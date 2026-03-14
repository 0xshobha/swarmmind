//! The AgentEconomy coordinator.
//!
//! Ties together all economy subsystems: task management, auctions,
//! execution, reputation, and recovery — into a single agent runtime.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

use vertex_core::identity::{AgentId, AgentProfile, AgentCapability};
use vertex_core::node::VertexNode;
use vertex_core::protocol::{MessageType, SwarmMessage};
use vertex_core::state::SwarmState;
use vertex_core::handshake::HandshakeManager;

use crate::auction::AuctionEngine;
use crate::execution::TaskExecutor;
use crate::reputation::ReputationEngine;
use crate::task::TaskManager;
use crate::recovery::RecoveryManager;

/// The full agent economy runtime.
pub struct AgentEconomy {
    pub agent_id: AgentId,
    pub profile: AgentProfile,
    pub node: Arc<VertexNode>,
    pub state: Arc<RwLock<SwarmState>>,
    pub handshake: Arc<HandshakeManager>,
    pub task_manager: Arc<TaskManager>,
    pub auction_engine: Arc<AuctionEngine>,
    pub executor: Arc<TaskExecutor>,
    pub reputation: Arc<ReputationEngine>,
}

impl AgentEconomy {
    pub fn new(
        profile: AgentProfile,
        node: Arc<VertexNode>,
    ) -> Self {
        let agent_id = profile.id.clone();
        let state = Arc::new(RwLock::new(SwarmState::new()));

        let handshake = Arc::new(HandshakeManager::new(
            profile.clone(),
            Arc::clone(&node),
            Arc::clone(&state),
        ));

        let task_manager = Arc::new(TaskManager::new(
            agent_id.clone(),
            Arc::clone(&node),
        ));

        let auction_engine = Arc::new(AuctionEngine::new(
            agent_id.clone(),
            Arc::clone(&node),
            Arc::clone(&state),
        ));

        let executor = Arc::new(TaskExecutor::new(
            agent_id.clone(),
            Arc::clone(&node),
        ));

        let reputation = Arc::new(ReputationEngine::new(
            agent_id.clone(),
            Arc::clone(&node),
        ));

        Self {
            agent_id,
            profile,
            node,
            state,
            handshake,
            task_manager,
            auction_engine,
            executor,
            reputation,
        }
    }

    /// Start the full agent economy lifecycle.
    pub async fn start(&self) -> anyhow::Result<()> {
        info!(agent = %self.agent_id, name = %self.profile.name, "🧠 SwarmMind Agent Economy starting");

        // 1. Register ourselves in local state
        {
            let mut state = self.state.write().await;
            let hello_msg = SwarmMessage::new(
                self.agent_id.clone(),
                0,
                MessageType::Hello {
                    profile: self.profile.clone(),
                },
            );
            state.apply_message(&hello_msg);
        }

        // 2. Send HELLO handshake
        self.handshake.send_hello().await?;

        // 3. Start heartbeats
        self.handshake.start_heartbeats();

        // 4. Start stale-peer detection
        self.handshake.start_stale_detection();

        // 5. Start auto auction resolution
        self.auction_engine.start_auto_resolve();

        // 6. Start recovery monitoring
        let recovery = Arc::new(RecoveryManager::new(
            self.agent_id.clone(),
            Arc::clone(&self.state),
            Arc::clone(&self.auction_engine),
            Arc::clone(&self.reputation),
        ));
        recovery.start_monitoring();

        info!(agent = %self.agent_id, "✅ Agent economy fully initialized");
        Ok(())
    }

    /// Process an incoming consensus message.
    pub async fn handle_message(&self, msg: SwarmMessage) {
        // Update replicated state
        {
            let mut state = self.state.write().await;
            state.apply_message(&msg);
        }

        // Handle economy-specific messages
        match &msg.msg_type {
            MessageType::TaskBroadcast { task } => {
                // Open auction for new task
                self.auction_engine.open_auction(task.clone()).await;

                // Check if we should bid
                if msg.sender != self.agent_id {
                    if self.auction_engine.should_bid(task).await {
                        // Calculate bid price based on complexity and our reputation
                        let state = self.state.read().await;
                        let my_rep = state.get_reputation(&self.agent_id);
                        drop(state);

                        let price = (task.complexity as u64) * 10 + (100 - my_rep.min(100) as u64);
                        let estimated_time = (task.complexity as u64) * 2000;

                        if let Err(e) = self
                            .auction_engine
                            .submit_bid(&task.id, price, estimated_time)
                            .await
                        {
                            warn!("Failed to submit bid: {e}");
                        }
                    }
                }
            }

            MessageType::TaskBid { task_id, bid } => {
                self.auction_engine
                    .record_bid(task_id, msg.sender.clone(), bid.clone())
                    .await;
            }

            MessageType::TaskAllocated { task_id, winner } => {
                if *winner == self.agent_id {
                    info!(task_id = %task_id, "🎯 I won the auction! Starting execution...");

                    // Get task complexity from state
                    let state = self.state.read().await;
                    let complexity = state
                        .task_statuses
                        .get(task_id)
                        .map(|_| 5u8) // default complexity
                        .unwrap_or(5);
                    drop(state);

                    let executor = Arc::clone(&self.executor);
                    let reputation = Arc::clone(&self.reputation);
                    let agent_id = self.agent_id.clone();
                    let tid = task_id.clone();

                    // Execute task in background
                    tokio::spawn(async move {
                        match executor.execute_task(&tid, complexity).await {
                            Ok(()) => {
                                let _ = reputation.award_success(&agent_id, true);
                            }
                            Err(e) => {
                                warn!("Task execution failed: {e}");
                                let _ = reputation.penalize_failure(&agent_id);
                            }
                        }
                    });
                }
            }

            MessageType::TaskCompleted { task_id, .. } => {
                if msg.sender != self.agent_id {
                    debug!(task_id = %task_id, agent = %msg.sender, "Peer completed task");
                }
            }

            _ => {}
        }
    }

    /// Get the current swarm state as JSON (for dashboard).
    pub async fn state_json(&self) -> String {
        self.state.read().await.to_json()
    }
}
