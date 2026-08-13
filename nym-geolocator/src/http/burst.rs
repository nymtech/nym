// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::error::RequestError;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;

/// How many measurements a node may ask for that tell us nothing new, before it has to wait.
///
/// Every node-requested measurement costs a metered lookup and a chain transaction, so a node that
/// keeps asking and keeps being in the same place is spending a shared resource to no end. The
/// counter is of *consecutive unchanged* results rather than of requests, so a node that genuinely
/// relocates is never limited for having moved.
///
/// The limit is per agent. Each deployment holds its own counters, so a node's effective allowance
/// across a fleet is this threshold times the number of agents, which is accepted.
#[derive(Clone)]
pub(crate) struct BurstLimiter {
    threshold: u32,

    cooldown: Duration,

    nodes: Arc<Mutex<HashMap<NodeId, NodeBurstState>>>,
}

#[derive(Default)]
struct NodeBurstState {
    consecutive_unchanged: u32,

    cooldown_until: Option<OffsetDateTime>,
}

impl BurstLimiter {
    pub(crate) fn new(threshold: u32, cooldown: Duration) -> BurstLimiter {
        BurstLimiter {
            threshold,
            cooldown,
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Take a node's allowance for one measurement, rejecting it if the allowance is spent.
    ///
    /// The claim is charged here rather than once the measurement returns, under the same lock as
    /// the check, because a counter only advanced afterwards is passed by every concurrent request
    /// at once: a node can sign one request per second across the replay window and deliver them
    /// together, and each would find the counter untouched. Charging under the lock bounds a node
    /// to `threshold` measurements in flight however they arrive.
    ///
    /// Deliberately not called for the regular sweep or for a bearer-token request, so that the
    /// service's own activity can never lock a node out of asking.
    pub(crate) async fn claim_allowance(&self, node_id: NodeId) -> Result<(), RequestError> {
        let mut nodes = self.nodes.lock().await;
        let state = nodes.entry(node_id).or_default();

        if let Some(cooldown_until) = state.cooldown_until {
            if OffsetDateTime::now_utc() < cooldown_until {
                return Err(RequestError::too_many_requests(format!(
                    "node {node_id} has spent its re-test allowance; further requests will be accepted after {cooldown_until}"
                )));
            }

            // the cooldown has run out, which restores the allowance rather than merely permitting
            // one more request: leaving the counter at the threshold would put the node straight
            // back into cooldown on its next unchanged result
            *state = NodeBurstState::default();
        }

        state.consecutive_unchanged += 1;
        if state.consecutive_unchanged >= self.threshold {
            state.cooldown_until = Some(OffsetDateTime::now_utc() + self.cooldown);
        }

        Ok(())
    }

    /// Give back a claim taken for a measurement that never happened.
    ///
    /// A failed request spent neither the metered lookup nor the transaction the allowance exists
    /// to protect, so it must not be charged: with a threshold of three and a cooldown of a week,
    /// an outage on our side would otherwise lock out every node that asked during it.
    ///
    /// A request abandoned before this runs, by a client that disconnected mid-measurement, keeps
    /// its claim. That errs towards charging rather than towards letting a node drop connections
    /// to measure for free.
    pub(crate) async fn release_claim(&self, node_id: NodeId) {
        let mut nodes = self.nodes.lock().await;
        let Some(state) = nodes.get_mut(&node_id) else {
            return;
        };

        state.consecutive_unchanged = state.consecutive_unchanged.saturating_sub(1);

        // the claim being returned may be the one that tripped the cooldown, and a cooldown left
        // standing for a measurement that never happened is exactly what this exists to avoid
        if state.consecutive_unchanged < self.threshold {
            state.cooldown_until = None;
        }
    }

    /// Return a node's whole allowance, its measurement having produced a changed location.
    ///
    /// A node that has actually moved gets everything back at once, so relocating is never what
    /// puts it into cooldown. An unchanged result needs nothing recorded here: the claim already
    /// counted it.
    pub(crate) async fn restore_allowance(&self, node_id: NodeId) {
        // absent and default are the same state to every reader, so this is a reset
        self.nodes.lock().await.remove(&node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THRESHOLD: u32 = 3;
    const COOLDOWN: Duration = Duration::from_secs(7 * 24 * 60 * 60);

    fn limiter() -> BurstLimiter {
        BurstLimiter::new(THRESHOLD, COOLDOWN)
    }

    /// `count` requests that were served and whose measurements produced no change, which is the
    /// claim alone: nothing is recorded for an unchanged result.
    async fn unchanged_measurements(limiter: &BurstLimiter, node_id: NodeId, count: u32) {
        for _ in 0..count {
            assert!(limiter.claim_allowance(node_id).await.is_ok());
        }
    }

    #[tokio::test]
    async fn a_node_that_has_never_asked_is_allowed() {
        assert!(limiter().claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn the_allowance_survives_one_short_of_the_threshold() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;

        assert!(limiter.claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn the_threshold_of_unchanged_results_triggers_the_cooldown() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;

        assert!(limiter.claim_allowance(42).await.is_err());
    }

    #[tokio::test]
    async fn measurements_still_in_flight_count_against_the_allowance() {
        // the property the whole claim-before-measuring order exists for. none of these requests
        // has produced a result yet, so a limiter that only counted completed measurements would
        // let every one of them through at once - and a node can deliver a whole replay window's
        // worth of distinct requests simultaneously
        let limiter = limiter();
        for _ in 0..THRESHOLD {
            assert!(limiter.claim_allowance(42).await.is_ok());
        }

        assert!(limiter.claim_allowance(42).await.is_err());
    }

    #[tokio::test]
    async fn a_released_claim_does_not_count() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;

        limiter.release_claim(42).await;

        // the released claim gave back exactly one, so there are two left rather than none
        unchanged_measurements(&limiter, 42, 1).await;
        assert!(limiter.claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn releasing_the_claim_that_tripped_the_cooldown_lifts_it() {
        // an upstream failure on the request that happened to be the last of the allowance must
        // not leave a week-long cooldown behind for a measurement that never took place
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.claim_allowance(42).await.is_err());

        limiter.release_claim(42).await;

        assert!(limiter.claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn a_release_with_nothing_to_give_back_does_nothing() {
        let limiter = limiter();

        // a node that never asked in the first place
        limiter.release_claim(42).await;

        // and one whose claims have all been given back already
        assert!(limiter.claim_allowance(42).await.is_ok());
        limiter.release_claim(42).await;
        limiter.release_claim(42).await;

        // neither handed out a claim in the negative, which would widen the allowance instead
        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.claim_allowance(42).await.is_err());
    }

    #[tokio::test]
    async fn a_changed_result_restores_the_allowance() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;

        limiter.restore_allowance(42).await;

        // the counter is back to zero rather than merely below the threshold, so a full further
        // run of unchanged results is needed before the node is limited again
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;
        assert!(limiter.claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn a_changed_result_ends_an_active_cooldown() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.claim_allowance(42).await.is_err());

        limiter.restore_allowance(42).await;

        assert!(limiter.claim_allowance(42).await.is_ok());
    }

    #[tokio::test]
    async fn an_elapsed_cooldown_restores_the_whole_allowance() {
        // a real cooldown short enough to wait out. it cannot be zero-length: the second half of
        // this test needs the *re-triggered* cooldown to still be in force, and a zero-length one
        // would have expired by the time it was checked, passing whether the counter reset or not
        let cooldown = Duration::from_millis(150);
        let limiter = BurstLimiter::new(THRESHOLD, cooldown);

        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.claim_allowance(42).await.is_err());

        tokio::time::sleep(cooldown + Duration::from_millis(50)).await;

        // the counter came back with the allowance rather than the node merely being let through
        // once, so a full further threshold of unchanged results is needed before it is limited
        // again. each of these asserts it was allowed, which is what makes that bite
        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.claim_allowance(42).await.is_err());
    }

    #[tokio::test]
    async fn one_node_cannot_spend_another_nodes_allowance() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;

        assert!(limiter.claim_allowance(43).await.is_ok());
    }
}
