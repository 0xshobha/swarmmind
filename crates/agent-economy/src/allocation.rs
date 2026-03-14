//! Consensus-based task allocation.
//!
//! After an auction resolves, the winning bid is deterministically
//! computed by every node in the network from the consensus-ordered
//! bid sequence. No coordinator needed.

use vertex_core::AgentId;
use vertex_core::protocol::BidInfo;
use tracing::info;

/// Deterministic allocation result.
#[derive(Debug, Clone)]
pub struct AllocationResult {
    pub task_id: String,
    pub winner: AgentId,
    pub winning_bid: BidInfo,
    pub runner_up: Option<AgentId>,
}

/// Given consensus-ordered bids, deterministically select the winner.
/// Every node in the swarm computes the same result.
pub fn allocate_task(
    task_id: &str,
    bids: &[(AgentId, BidInfo)],
) -> Option<AllocationResult> {
    if bids.is_empty() {
        return None;
    }

    let mut scored: Vec<(usize, f64)> = bids
        .iter()
        .enumerate()
        .map(|(i, (_, bid))| (i, compute_allocation_score(bid)))
        .collect();

    // Sort by score descending, then by index for deterministic tie-breaking
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let winner_idx = scored[0].0;
    let runner_up = scored.get(1).map(|(idx, _)| bids[*idx].0.clone());

    let result = AllocationResult {
        task_id: task_id.to_string(),
        winner: bids[winner_idx].0.clone(),
        winning_bid: bids[winner_idx].1.clone(),
        runner_up,
    };

    info!(
        task_id = %task_id,
        winner = %result.winner,
        score = scored[0].1,
        total_bids = bids.len(),
        "📋 Task allocated deterministically"
    );

    Some(result)
}

/// Deterministic score: weighted combination of reputation, capability match, price, and speed.
fn compute_allocation_score(bid: &BidInfo) -> f64 {
    let rep = (bid.reputation as f64).max(1.0) / 100.0;
    let cap = bid.capability_match_score;
    let price = 1000.0 / (bid.price as f64).max(1.0);
    let speed = 10000.0 / (bid.estimated_time_ms as f64).max(1.0);

    rep * 0.4 + cap * 0.3 + price * 0.2 + speed * 0.1
}
