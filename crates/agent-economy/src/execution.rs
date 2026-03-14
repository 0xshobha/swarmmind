//! Task execution tracking.
//!
//! Simulates task execution by the winning agent, broadcasting
//! progress updates and completion/failure through Vertex consensus.

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use vertex_core::node::VertexNode;
use vertex_core::protocol::{MessageType, SwarmMessage};
use vertex_core::AgentId;

/// Simulates executing a task with progress updates.
pub struct TaskExecutor {
    agent_id: AgentId,
    node: Arc<VertexNode>,
    seq: std::sync::atomic::AtomicU64,
}

impl TaskExecutor {
    pub fn new(agent_id: AgentId, node: Arc<VertexNode>) -> Self {
        Self {
            agent_id,
            node,
            seq: std::sync::atomic::AtomicU64::new(3000),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Execute a task (simulated) with progress reporting.
    pub async fn execute_task(&self, task_id: &str, complexity: u8) -> anyhow::Result<()> {
        info!(task_id = %task_id, complexity = complexity, "⚙️ Starting task execution");

        let steps = complexity.max(2) as u32;
        let step_duration_ms = 1500; // 1.5s per step

        for step in 1..=steps {
            sleep(Duration::from_millis(step_duration_ms)).await;

            let progress_pct = ((step as f64 / steps as f64) * 100.0) as u8;
            let stage = match progress_pct {
                0..=25 => "Initializing",
                26..=50 => "Processing",
                51..=75 => "Computing",
                76..=99 => "Finalizing",
                _ => "Complete",
            };

            let msg = SwarmMessage::new(
                self.agent_id.clone(),
                self.next_seq(),
                MessageType::TaskProgress {
                    task_id: task_id.to_string(),
                    progress_pct,
                    message: format!("{stage} (step {step}/{steps})"),
                },
            );

            self.node.send_message(&msg)?;
            info!(
                task_id = %task_id,
                progress = progress_pct,
                stage = %stage,
                "📊 Progress update sent"
            );
        }

        // Simulate result hash
        let result_hash = format!("0x{:x}", chrono::Utc::now().timestamp_millis());

        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            self.next_seq(),
            MessageType::TaskCompleted {
                task_id: task_id.to_string(),
                result_hash: result_hash.clone(),
            },
        );

        self.node.send_message(&msg)?;
        info!(task_id = %task_id, hash = %result_hash, "✅ Task execution completed");

        Ok(())
    }

    /// Report task failure.
    pub async fn report_failure(&self, task_id: &str, reason: &str) -> anyhow::Result<()> {
        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            self.next_seq(),
            MessageType::TaskFailed {
                task_id: task_id.to_string(),
                reason: reason.to_string(),
            },
        );

        self.node.send_message(&msg)?;
        warn!(task_id = %task_id, reason = %reason, "❌ Task failure reported");
        Ok(())
    }
}
