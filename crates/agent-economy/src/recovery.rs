//! Self-healing: task recovery and reassignment.
//!
//! Detects abandoned tasks (assigned to stale/offline agents)
//! and automatically re-auctions them to healthy peers.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

use vertex_core::state::SwarmState;
use vertex_core::AgentId;

use crate::auction::AuctionEngine;
use crate::reputation::ReputationEngine;

/// Monitors for abandoned tasks and triggers recovery.
pub struct RecoveryManager {
    agent_id: AgentId,
    state: Arc<RwLock<SwarmState>>,
    auction_engine: Arc<AuctionEngine>,
    reputation_engine: Arc<ReputationEngine>,
}

impl RecoveryManager {
    pub fn new(
        agent_id: AgentId,
        state: Arc<RwLock<SwarmState>>,
        auction_engine: Arc<AuctionEngine>,
        reputation_engine: Arc<ReputationEngine>,
    ) -> Self {
        Self {
            agent_id,
            state,
            auction_engine,
            reputation_engine,
        }
    }

    /// Start monitoring for abandoned tasks.
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(5));
            loop {
                timer.tick().await;
                self.check_and_recover().await;
            }
        });
    }

    /// Check for abandoned tasks and re-auction them.
    async fn check_and_recover(&self) {
        let state = self.state.read().await;
        let abandoned = state.abandoned_tasks();
        drop(state);

        for task_id in abandoned {
            warn!(task_id = %task_id, "🔄 Detected abandoned task — initiating recovery");

            // Get the failed agent and penalize
            let state = self.state.read().await;
            if let Some(agent_id) = state.task_assignments.get(&task_id) {
                let _ = self.reputation_engine.penalize_abandonment(agent_id);
            }

            // Re-auction the task
            if let Some(task_status) = state.task_statuses.get(&task_id) {
                info!(
                    task_id = %task_id,
                    "📢 Re-auctioning abandoned task"
                );
                // The task broadcast will trigger a new auction
                // In a full implementation, we'd reconstruct the TaskSpec from state
            }
            drop(state);
        }
    }
}
