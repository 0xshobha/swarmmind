//! Task definition and broadcasting.

use std::sync::Arc;
use uuid::Uuid;
use tracing::info;

use vertex_core::identity::AgentCapability;
use vertex_core::node::VertexNode;
use vertex_core::protocol::{MessageType, SwarmMessage, TaskSpec};
use vertex_core::AgentId;

/// Manages task creation and broadcasting.
pub struct TaskManager {
    agent_id: AgentId,
    node: Arc<VertexNode>,
    seq: std::sync::atomic::AtomicU64,
}

impl TaskManager {
    pub fn new(agent_id: AgentId, node: Arc<VertexNode>) -> Self {
        Self {
            agent_id,
            node,
            seq: std::sync::atomic::AtomicU64::new(1000), // task seqs start at 1000
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Create and broadcast a new task to the swarm.
    pub fn broadcast_task(
        &self,
        description: &str,
        required_capabilities: Vec<AgentCapability>,
        complexity: u8,
        reward: u64,
        deadline_secs: u64,
    ) -> anyhow::Result<String> {
        let task_id = format!("task-{}", &Uuid::new_v4().to_string()[..8]);
        let deadline_ms = chrono::Utc::now().timestamp_millis() as u64 + (deadline_secs * 1000);

        let task = TaskSpec {
            id: task_id.clone(),
            description: description.to_string(),
            required_capabilities,
            complexity,
            reward,
            deadline_ms,
            decomposable: complexity > 7,
        };

        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            self.next_seq(),
            MessageType::TaskBroadcast { task },
        );

        self.node.send_message(&msg)?;
        info!(task_id = %task_id, desc = %description, reward = reward, "📢 Task broadcast to swarm");
        Ok(task_id)
    }
}
