//! Wire protocol: all messages exchanged through Vertex consensus.
//!
//! Every transaction sent through the Vertex engine is a serialized `SwarmMessage`.
//! The `MessageType` enum determines how the receiving node processes it.

use serde::{Deserialize, Serialize};
use crate::identity::{AgentId, AgentProfile, AgentCapability};

/// Top-level message type for all SwarmMind protocol communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmMessage {
    /// Sender's agent ID.
    pub sender: AgentId,
    /// Monotonically increasing sequence number from this sender.
    pub seq: u64,
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
    /// The actual message payload.
    pub msg_type: MessageType,
}

impl SwarmMessage {
    pub fn new(sender: AgentId, seq: u64, msg_type: MessageType) -> Self {
        let timestamp_ms = chrono::Utc::now().timestamp_millis() as u64;
        Self {
            sender,
            seq,
            timestamp_ms,
            msg_type,
        }
    }

    /// Serialize to bytes for Vertex transaction.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("SwarmMessage serialization should not fail")
    }

    /// Deserialize from Vertex transaction bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// All possible message types in the SwarmMind protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    // ── Handshake & Heartbeat (Warm-Up) ──
    /// Initial hello with agent profile.
    Hello { profile: AgentProfile },
    /// Periodic heartbeat with current status.
    Heartbeat { status: AgentStatus },
    /// Role change announcement.
    RoleChange { new_role: String },

    // ── Agent Economy (Track 3) ──
    /// Broadcast a new task to the swarm.
    TaskBroadcast { task: TaskSpec },
    /// Submit a bid for a task.
    TaskBid { task_id: String, bid: BidInfo },
    /// Announce auction result (deterministic from consensus-ordered bids).
    TaskAllocated { task_id: String, winner: AgentId },
    /// Report task progress.
    TaskProgress { task_id: String, progress_pct: u8, message: String },
    /// Task completed successfully.
    TaskCompleted { task_id: String, result_hash: String },
    /// Task failed.
    TaskFailed { task_id: String, reason: String },
    /// Reputation update after task outcome.
    ReputationUpdate { agent: AgentId, delta: i32, reason: String },
}

/// Current agent status, broadcast in heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub role: String,
    pub active_tasks: u32,
    pub reputation: i64,
    pub cpu_load_pct: u8,
    pub status: PeerStatus,
}

/// Peer connectivity/health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerStatus {
    Active,
    Busy,
    Stale,
    Offline,
}

impl std::fmt::Display for PeerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "ACTIVE"),
            Self::Busy => write!(f, "BUSY"),
            Self::Stale => write!(f, "STALE"),
            Self::Offline => write!(f, "OFFLINE"),
        }
    }
}

/// Specification for a task to be auctioned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: String,
    pub description: String,
    pub required_capabilities: Vec<AgentCapability>,
    pub complexity: u8, // 1-10
    pub reward: u64,    // virtual currency units
    pub deadline_ms: u64,
    pub decomposable: bool, // can be split among multiple agents
}

/// A bid submitted by an agent for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidInfo {
    pub price: u64,
    pub estimated_time_ms: u64,
    pub capability_match_score: f64,
    pub reputation: i64,
}
