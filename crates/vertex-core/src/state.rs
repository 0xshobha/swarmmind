//! Replicated swarm state.
//!
//! Maintains a local view of every peer's status, reputation, and role.
//! State is updated via consensus-ordered messages from Vertex.

use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

use crate::identity::{AgentCapability, AgentId};
use crate::protocol::{PeerStatus, MessageType, SwarmMessage};

/// State of a single peer in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerState {
    pub peer_id: AgentId,
    pub name: String,
    pub last_seen_ms: u64,
    pub role: String,
    pub status: PeerStatus,
    pub capabilities: Vec<AgentCapability>,
    pub reputation: i64,
    pub active_tasks: u32,
    pub was_stale: bool,
}

/// The full replicated state of the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    pub peers: HashMap<String, PeerState>,
    pub task_assignments: HashMap<String, AgentId>, // task_id -> assigned agent
    pub task_statuses: HashMap<String, TaskStatus>,
}

/// Task status in the global state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task_id: String,
    pub assigned_to: Option<AgentId>,
    pub progress_pct: u8,
    pub status: TaskLifecycle,
    pub created_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskLifecycle {
    Auctioning,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Abandoned,
}

impl SwarmState {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            task_assignments: HashMap::new(),
            task_statuses: HashMap::new(),
        }
    }

    /// Process an incoming consensus-ordered message and update state.
    pub fn apply_message(&mut self, msg: &SwarmMessage) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        let peer_key = msg.sender.0.clone();

        match &msg.msg_type {
            MessageType::Hello { profile } => {
                let entry = self.peers.entry(peer_key.clone()).or_insert_with(|| PeerState {
                    peer_id: msg.sender.clone(),
                    name: profile.name.clone(),
                    last_seen_ms: now_ms,
                    role: "worker".to_string(),
                    status: PeerStatus::Active,
                    capabilities: profile.capabilities.clone(),
                    reputation: 100, // start with base reputation
                    active_tasks: 0,
                    was_stale: false,
                });
                entry.last_seen_ms = now_ms;
                entry.status = PeerStatus::Active;
                entry.capabilities = profile.capabilities.clone();
                entry.name = profile.name.clone();
                if entry.was_stale {
                    entry.was_stale = false;
                    info!(peer = %peer_key, "Peer recovered from stale state via HELLO");
                }
                info!(peer = %peer_key, name = %profile.name, "📋 Peer registered in swarm state");
            }

            MessageType::Heartbeat { status } => {
                if let Some(peer) = self.peers.get_mut(&peer_key) {
                    peer.last_seen_ms = now_ms;
                    peer.role = status.role.clone();
                    peer.active_tasks = status.active_tasks;
                    peer.status = status.status.clone();
                    if peer.was_stale {
                        peer.was_stale = false;
                        peer.status = PeerStatus::Active;
                    }
                } else {
                    // New peer seen via heartbeat before HELLO
                    self.peers.insert(peer_key.clone(), PeerState {
                        peer_id: msg.sender.clone(),
                        name: format!("agent-{}", &peer_key[..6]),
                        last_seen_ms: now_ms,
                        role: status.role.clone(),
                        status: PeerStatus::Active,
                        capabilities: vec![],
                        reputation: status.reputation,
                        active_tasks: status.active_tasks,
                        was_stale: false,
                    });
                }
                debug!(peer = %peer_key, role = %status.role, "💓 Heartbeat received");
            }

            MessageType::RoleChange { new_role } => {
                if let Some(peer) = self.peers.get_mut(&peer_key) {
                    let old_role = peer.role.clone();
                    peer.role = new_role.clone();
                    peer.last_seen_ms = now_ms;
                    info!(
                        peer = %peer_key,
                        old = %old_role,
                        new = %new_role,
                        "🔄 Role change mirrored"
                    );
                }
            }

            MessageType::TaskAllocated { task_id, winner } => {
                self.task_assignments.insert(task_id.clone(), winner.clone());
                if let Some(ts) = self.task_statuses.get_mut(task_id) {
                    ts.assigned_to = Some(winner.clone());
                    ts.status = TaskLifecycle::Assigned;
                }
                if let Some(peer) = self.peers.get_mut(&winner.0) {
                    peer.active_tasks += 1;
                }
                info!(task = %task_id, winner = %winner, "📌 Task allocated");
            }

            MessageType::TaskProgress { task_id, progress_pct, message } => {
                if let Some(ts) = self.task_statuses.get_mut(task_id) {
                    ts.progress_pct = *progress_pct;
                    ts.status = TaskLifecycle::InProgress;
                }
                debug!(task = %task_id, progress = progress_pct, "📊 Task progress updated");
            }

            MessageType::TaskCompleted { task_id, result_hash } => {
                if let Some(ts) = self.task_statuses.get_mut(task_id) {
                    ts.progress_pct = 100;
                    ts.status = TaskLifecycle::Completed;
                }
                if let Some(winner) = self.task_assignments.get(task_id) {
                    if let Some(peer) = self.peers.get_mut(&winner.0) {
                        peer.active_tasks = peer.active_tasks.saturating_sub(1);
                    }
                }
                info!(task = %task_id, hash = %result_hash, "✅ Task completed");
            }

            MessageType::TaskFailed { task_id, reason } => {
                if let Some(ts) = self.task_statuses.get_mut(task_id) {
                    ts.status = TaskLifecycle::Failed;
                }
                if let Some(winner) = self.task_assignments.get(task_id) {
                    if let Some(peer) = self.peers.get_mut(&winner.0) {
                        peer.active_tasks = peer.active_tasks.saturating_sub(1);
                    }
                }
                warn!(task = %task_id, reason = %reason, "❌ Task failed");
            }

            MessageType::ReputationUpdate { agent, delta, reason } => {
                if let Some(peer) = self.peers.get_mut(&agent.0) {
                    peer.reputation += *delta as i64;
                    info!(
                        agent = %agent,
                        delta = delta,
                        new_rep = peer.reputation,
                        reason = %reason,
                        "⭐ Reputation updated"
                    );
                }
            }

            MessageType::TaskBroadcast { task } => {
                self.task_statuses.insert(task.id.clone(), TaskStatus {
                    task_id: task.id.clone(),
                    assigned_to: None,
                    progress_pct: 0,
                    status: TaskLifecycle::Auctioning,
                    created_ms: now_ms,
                });
                info!(task_id = %task.id, desc = %task.description, "📢 New task broadcast");
            }

            MessageType::TaskBid { .. } => {
                // Bids are processed by the auction module, not state directly
                debug!("Bid received (handled by auction engine)");
            }
        }
    }

    /// Detect peers whose heartbeat is older than `threshold_ms`.
    pub fn detect_stale_peers(&mut self, threshold_ms: u64) -> Vec<AgentId> {
        let now = Utc::now().timestamp_millis() as u64;
        let mut stale = vec![];
        for peer in self.peers.values_mut() {
            if peer.status != PeerStatus::Stale
                && peer.status != PeerStatus::Offline
                && now.saturating_sub(peer.last_seen_ms) > threshold_ms
            {
                peer.status = PeerStatus::Stale;
                peer.was_stale = true;
                stale.push(peer.peer_id.clone());
            }
        }
        stale
    }

    /// Detect peers that were stale but have since recovered.
    pub fn detect_recovered_peers(&mut self) -> Vec<AgentId> {
        let mut recovered = vec![];
        for peer in self.peers.values_mut() {
            if peer.was_stale && peer.status == PeerStatus::Active {
                peer.was_stale = false;
                recovered.push(peer.peer_id.clone());
            }
        }
        recovered
    }

    /// Get number of active tasks for a given agent.
    pub fn my_active_task_count(&self, agent_id: &AgentId) -> u32 {
        self.peers
            .get(&agent_id.0)
            .map(|p| p.active_tasks)
            .unwrap_or(0)
    }

    /// Get reputation for a given agent.
    pub fn get_reputation(&self, agent_id: &AgentId) -> i64 {
        self.peers
            .get(&agent_id.0)
            .map(|p| p.reputation)
            .unwrap_or(100)
    }

    /// Get all active (non-stale, non-offline) peers.
    pub fn active_peers(&self) -> Vec<&PeerState> {
        self.peers
            .values()
            .filter(|p| p.status == PeerStatus::Active || p.status == PeerStatus::Busy)
            .collect()
    }

    /// Get tasks assigned to agents that are now stale/offline.
    pub fn abandoned_tasks(&self) -> Vec<String> {
        let mut abandoned = vec![];
        for (task_id, agent_id) in &self.task_assignments {
            if let Some(ts) = self.task_statuses.get(task_id) {
                if ts.status == TaskLifecycle::Assigned || ts.status == TaskLifecycle::InProgress {
                    if let Some(peer) = self.peers.get(&agent_id.0) {
                        if peer.status == PeerStatus::Stale || peer.status == PeerStatus::Offline {
                            abandoned.push(task_id.clone());
                        }
                    }
                }
            }
        }
        abandoned
    }

    /// Serialize state snapshot to JSON for the dashboard.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

impl Default for SwarmState {
    fn default() -> Self {
        Self::new()
    }
}
