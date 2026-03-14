//! Decentralized sealed-bid auction protocol.
//!
//! When a task is broadcast, agents who can handle it submit bids
//! via Vertex transactions. Fair ordering by Vertex prevents front-running.
//! After the auction window closes, all nodes deterministically compute
//! the same winner from the consensus-ordered bid sequence.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{info, debug, warn};

use vertex_core::node::VertexNode;
use vertex_core::protocol::{BidInfo, MessageType, SwarmMessage, TaskSpec};
use vertex_core::state::SwarmState;
use vertex_core::AgentId;

/// Collects bids for a task and determines winner.
#[derive(Debug, Clone)]
pub struct Auction {
    pub task_id: String,
    pub task: TaskSpec,
    pub bids: Vec<(AgentId, BidInfo)>,
    pub deadline_ms: u64,
    pub resolved: bool,
    pub winner: Option<AgentId>,
}

/// Manages all active auctions.
pub struct AuctionEngine {
    agent_id: AgentId,
    node: Arc<VertexNode>,
    state: Arc<RwLock<SwarmState>>,
    auctions: Arc<RwLock<HashMap<String, Auction>>>,
    seq: std::sync::atomic::AtomicU64,
}

impl AuctionEngine {
    pub fn new(
        agent_id: AgentId,
        node: Arc<VertexNode>,
        state: Arc<RwLock<SwarmState>>,
    ) -> Self {
        Self {
            agent_id,
            node,
            state,
            auctions: Arc::new(RwLock::new(HashMap::new())),
            seq: std::sync::atomic::AtomicU64::new(2000),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Register a new task for auction.
    pub async fn open_auction(&self, task: TaskSpec) {
        let auction = Auction {
            task_id: task.id.clone(),
            task: task.clone(),
            bids: vec![],
            deadline_ms: chrono::Utc::now().timestamp_millis() as u64 + 5000, // 5s auction window
            resolved: false,
            winner: None,
        };
        self.auctions.write().await.insert(task.id.clone(), auction);
        info!(task_id = %task.id, "🔨 Auction opened (5s window)");
    }

    /// Submit a bid for a task (called when this agent wants to bid).
    pub async fn submit_bid(
        &self,
        task_id: &str,
        price: u64,
        estimated_time_ms: u64,
    ) -> anyhow::Result<()> {
        let state = self.state.read().await;
        let reputation = state.get_reputation(&self.agent_id);
        drop(state);

        let bid = BidInfo {
            price,
            estimated_time_ms,
            capability_match_score: 1.0, // we only bid on tasks we can handle
            reputation,
        };

        let msg = SwarmMessage::new(
            self.agent_id.clone(),
            self.next_seq(),
            MessageType::TaskBid {
                task_id: task_id.to_string(),
                bid: bid.clone(),
            },
        );

        self.node.send_message(&msg)?;
        info!(task_id = %task_id, price = price, "💰 Bid submitted");
        Ok(())
    }

    /// Record a bid from the consensus stream.
    pub async fn record_bid(&self, task_id: &str, bidder: AgentId, bid: BidInfo) {
        if let Some(auction) = self.auctions.write().await.get_mut(task_id) {
            if !auction.resolved {
                auction.bids.push((bidder.clone(), bid));
                debug!(task_id = %task_id, bidder = %bidder, "📝 Bid recorded");
            }
        }
    }

    /// Resolve an auction — deterministic winner selection.
    /// All nodes compute the same winner from consensus-ordered bids.
    pub async fn resolve_auction(&self, task_id: &str) -> Option<AgentId> {
        let mut auctions = self.auctions.write().await;
        if let Some(auction) = auctions.get_mut(task_id) {
            if auction.resolved || auction.bids.is_empty() {
                return auction.winner.clone();
            }

            // Deterministic scoring: reputation * cap_match / price * speed
            let winner = auction
                .bids
                .iter()
                .max_by(|(_, a), (_, b)| {
                    let score_a = compute_bid_score(a);
                    let score_b = compute_bid_score(b);
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(id, _)| id.clone());

            auction.resolved = true;
            auction.winner = winner.clone();

            if let Some(ref w) = winner {
                info!(
                    task_id = %task_id,
                    winner = %w,
                    total_bids = auction.bids.len(),
                    "🏆 Auction resolved"
                );

                // Broadcast allocation to swarm
                let msg = SwarmMessage::new(
                    self.agent_id.clone(),
                    self.next_seq(),
                    MessageType::TaskAllocated {
                        task_id: task_id.to_string(),
                        winner: w.clone(),
                    },
                );
                let _ = self.node.send_message(&msg);
            }

            winner
        } else {
            None
        }
    }

    /// Start automatic auction resolution after deadline.
    pub fn start_auto_resolve(self: &Arc<Self>) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(1)).await;
                let now = chrono::Utc::now().timestamp_millis() as u64;
                let tasks_to_resolve: Vec<String> = {
                    let auctions = this.auctions.read().await;
                    auctions
                        .values()
                        .filter(|a| !a.resolved && now >= a.deadline_ms)
                        .map(|a| a.task_id.clone())
                        .collect()
                };

                for task_id in tasks_to_resolve {
                    this.resolve_auction(&task_id).await;
                }
            }
        });
    }

    /// Check if this agent should bid on a given task.
    pub async fn should_bid(&self, task: &TaskSpec) -> bool {
        let state = self.state.read().await;
        // Check if we have the required capabilities
        if let Some(peer) = state.peers.get(&self.agent_id.0) {
            let can_handle = task
                .required_capabilities
                .iter()
                .all(|cap| peer.capabilities.contains(cap));
            let not_overloaded = peer.active_tasks < 3; // max concurrent tasks
            can_handle && not_overloaded
        } else {
            false
        }
    }
}

/// Compute a deterministic score for a bid.
/// Higher is better. All nodes compute the same score from the same data.
fn compute_bid_score(bid: &BidInfo) -> f64 {
    let reputation_factor = (bid.reputation as f64).max(1.0) / 100.0;
    let price_factor = 1000.0 / (bid.price as f64).max(1.0);
    let speed_factor = 10000.0 / (bid.estimated_time_ms as f64).max(1.0);
    let cap_factor = bid.capability_match_score;

    reputation_factor * 0.4 + cap_factor * 0.3 + price_factor * 0.2 + speed_factor * 0.1
}
