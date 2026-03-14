//! Agent reputation scoring.
//!
//! Reputation is replicated via BFT consensus. Scores influence
//! future auction outcomes, creating a merit-based economy.

use std::sync::Arc;
use tracing::info;

use vertex_core::node::VertexNode;
use vertex_core::protocol::{MessageType, SwarmMessage};
use vertex_core::AgentId;

/// Reputation score deltas for various outcomes.
pub const TASK_SUCCESS_BONUS: i32 = 10;
pub const TASK_FAILURE_PENALTY: i32 = -15;
pub const FAST_COMPLETION_BONUS: i32 = 5;
pub const ABANDONMENT_PENALTY: i32 = -25;

/// Manages reputation updates for the swarm.
pub struct ReputationEngine {
    agent_id: AgentId,
    node: Arc<VertexNode>,
    seq: std::sync::atomic::AtomicU64,
}

impl ReputationEngine {
    pub fn new(agent_id: AgentId, node: Arc<VertexNode>) -> Self {
        Self {
            agent_id,
            node,
            seq: std::sync::atomic::AtomicU64::new(4000),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Award reputation for successful task completion.
    pub fn award_success(&self, agent: &AgentId, was_fast: bool) -> anyhow::Result<()> {
        let delta = if was_fast {
            TASK_SUCCESS_BONUS + FAST_COMPLETION_BONUS
        } else {
            TASK_SUCCESS_BONUS
        };

        self.broadcast_update(agent, delta, "Task completed successfully")
    }

    /// Penalize for task failure.
    pub fn penalize_failure(&self, agent: &AgentId) -> anyhow::Result<()> {
        self.broadcast_update(agent, TASK_FAILURE_PENALTY, "Task execution failed")
    }

    /// Penalize for task abandonment (agent went stale).
    pub fn penalize_abandonment(&self, agent: &AgentId) -> anyhow::Result<()> {
        self.broadcast_update(agent, ABANDONMENT_PENALTY, "Task abandoned (agent offline)")
    }

    /// Broadcast a reputation update to the swarm.
    fn broadcast_update(&self, agent: &AgentId, delta: i32, reason: &str) -> anyhow::Result<()> {
        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            self.next_seq(),
            MessageType::ReputationUpdate {
                agent: agent.clone(),
                delta,
                reason: reason.to_string(),
            },
        );

        self.node.send_message(&msg)?;
        info!(
            target_agent = %agent,
            delta = delta,
            reason = %reason,
            "⭐ Reputation update broadcast"
        );
        Ok(())
    }
}
