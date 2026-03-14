//! Handshake and heartbeat protocol (Warm-Up: Stateful Handshake).
//!
//! Implements:
//! - HELLO handshake on startup with agent capabilities
//! - Periodic heartbeat transactions (every 3s)
//! - Role change propagation and mirroring (<1s)
//! - Stale peer detection (>10s without heartbeat)
//! - Recovery when stale peer returns

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use crate::identity::AgentProfile;
use crate::node::VertexNode;
use crate::protocol::{AgentStatus, MessageType, PeerStatus, SwarmMessage};
use crate::state::SwarmState;
use crate::AgentId;

/// Manages the handshake lifecycle for an agent.
pub struct HandshakeManager {
    agent_id: AgentId,
    profile: AgentProfile,
    node: Arc<VertexNode>,
    state: Arc<RwLock<SwarmState>>,
    seq: Arc<RwLock<u64>>,
    current_role: Arc<RwLock<String>>,
}

impl HandshakeManager {
    pub fn new(
        profile: AgentProfile,
        node: Arc<VertexNode>,
        state: Arc<RwLock<SwarmState>>,
    ) -> Self {
        let agent_id = profile.id.clone();
        Self {
            agent_id,
            profile,
            node,
            state,
            seq: Arc::new(RwLock::new(0)),
            current_role: Arc::new(RwLock::new("worker".to_string())),
        }
    }

    /// Get next sequence number.
    async fn next_seq(&self) -> u64 {
        let mut seq = self.seq.write().await;
        *seq += 1;
        *seq
    }

    /// Send the initial HELLO handshake message.
    pub async fn send_hello(&self) -> anyhow::Result<()> {
        let seq = self.next_seq().await;
        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            seq,
            MessageType::Hello {
                profile: self.profile.clone(),
            },
        );
        self.node.send_message(&msg)?;
        info!(
            agent = %self.agent_id,
            name = %self.profile.name,
            caps = ?self.profile.capabilities,
            "👋 HELLO handshake sent"
        );
        Ok(())
    }

    /// Start periodic heartbeats (every 3 seconds).
    pub fn start_heartbeats(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(3));
            loop {
                timer.tick().await;
                let seq = this.next_seq().await;
                let role = this.current_role.read().await.clone();
                let state = this.state.read().await;
                let active_tasks = state.my_active_task_count(&this.agent_id);
                let reputation = state.get_reputation(&this.agent_id);
                drop(state);

                let status = AgentStatus {
                    role: role.clone(),
                    active_tasks,
                    reputation,
                    cpu_load_pct: rand_cpu_load(),
                    status: PeerStatus::Active,
                };

                let msg = SwarmMessage::new(
                    this.agent_id.clone(),
                    seq,
                    MessageType::Heartbeat { status },
                );

                if let Err(e) = this.node.send_message(&msg) {
                    warn!("Failed to send heartbeat: {e}");
                }
            }
        });
    }

    /// Start stale-peer detection (check every 2 seconds, mark stale after 10s).
    pub fn start_stale_detection(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(2));
            loop {
                timer.tick().await;
                let mut state = this.state.write().await;
                let stale_peers = state.detect_stale_peers(10_000); // 10s threshold
                for peer_id in &stale_peers {
                    warn!(peer = %peer_id, "⚠️ Peer marked STALE (no heartbeat for >10s)");
                }
                let recovered = state.detect_recovered_peers();
                for peer_id in &recovered {
                    info!(peer = %peer_id, "✅ Peer RECOVERED and re-joined the swarm");
                }
            }
        });
    }

    /// Change this agent's role and broadcast to the swarm.
    pub async fn change_role(&self, new_role: &str) -> anyhow::Result<()> {
        {
            let mut role = self.current_role.write().await;
            *role = new_role.to_string();
        }
        let seq = self.next_seq().await;
        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            seq,
            MessageType::RoleChange {
                new_role: new_role.to_string(),
            },
        );
        self.node.send_message(&msg)?;
        info!(agent = %self.agent_id, role = new_role, "🔄 Role changed and broadcast");
        Ok(())
    }

    /// Get current role.
    pub async fn current_role(&self) -> String {
        self.current_role.read().await.clone()
    }
}

/// Simulated CPU load for demo purposes.
fn rand_cpu_load() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (t % 60 + 10) as u8 // 10-70% range
}
