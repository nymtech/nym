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

    /// Reject a node that has spent its allowance and whose cooldown has not yet elapsed.
    pub(crate) async fn ensure_allowed(&self, node_id: NodeId) -> Result<(), RequestError> {
        let mut nodes = self.nodes.lock().await;

        let Some(state) = nodes.get_mut(&node_id) else {
            return Ok(());
        };
        let Some(cooldown_until) = state.cooldown_until else {
            return Ok(());
        };

        if OffsetDateTime::now_utc() < cooldown_until {
            return Err(RequestError::too_many_requests(format!(
                "node {node_id} has spent its re-test allowance; further requests will be accepted after {cooldown_until}"
            )));
        }

        // the cooldown has run out, which restores the allowance rather than merely permitting one
        // more request: leaving the counter at the threshold would put the node straight back into
        // cooldown on its next unchanged result
        *state = NodeBurstState::default();

        Ok(())
    }

    /// Record what a *node-requested* measurement produced.
    ///
    /// Deliberately not called for the regular sweep or for a bearer-token request, so that the
    /// service's own activity can never lock a node out of asking.
    pub(crate) async fn record_measurement(&self, node_id: NodeId, changed: bool) {
        let mut nodes = self.nodes.lock().await;
        let state = nodes.entry(node_id).or_default();

        if changed {
            // a node that has actually moved gets its allowance back at once, so relocating is
            // never what puts it into cooldown
            *state = NodeBurstState::default();
            return;
        }

        state.consecutive_unchanged += 1;
        if state.consecutive_unchanged >= self.threshold {
            state.cooldown_until = Some(OffsetDateTime::now_utc() + self.cooldown);
        }
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

    async fn unchanged_measurements(limiter: &BurstLimiter, node_id: NodeId, count: u32) {
        for _ in 0..count {
            limiter.record_measurement(node_id, false).await;
        }
    }

    #[tokio::test]
    async fn a_node_that_has_never_asked_is_allowed() {
        assert!(limiter().ensure_allowed(42).await.is_ok());
    }

    #[tokio::test]
    async fn the_allowance_survives_one_short_of_the_threshold() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;

        assert!(limiter.ensure_allowed(42).await.is_ok());
    }

    #[tokio::test]
    async fn the_threshold_of_unchanged_results_triggers_the_cooldown() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;

        assert!(limiter.ensure_allowed(42).await.is_err());
    }

    #[tokio::test]
    async fn a_changed_result_restores_the_allowance() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;

        limiter.record_measurement(42, true).await;

        // the counter is back to zero rather than merely below the threshold, so a full further
        // run of unchanged results is needed before the node is limited again
        unchanged_measurements(&limiter, 42, THRESHOLD - 1).await;
        assert!(limiter.ensure_allowed(42).await.is_ok());
    }

    #[tokio::test]
    async fn a_changed_result_ends_an_active_cooldown() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.ensure_allowed(42).await.is_err());

        limiter.record_measurement(42, true).await;

        assert!(limiter.ensure_allowed(42).await.is_ok());
    }

    #[tokio::test]
    async fn an_elapsed_cooldown_restores_the_whole_allowance() {
        // a real cooldown short enough to wait out. it cannot be zero-length: the second half of
        // this test needs the *re-triggered* cooldown to still be in force, and a zero-length one
        // would have expired by the time it was checked, passing whether the counter reset or not
        let cooldown = Duration::from_millis(150);
        let limiter = BurstLimiter::new(THRESHOLD, cooldown);

        unchanged_measurements(&limiter, 42, THRESHOLD).await;
        assert!(limiter.ensure_allowed(42).await.is_err());

        tokio::time::sleep(cooldown + Duration::from_millis(50)).await;
        assert!(limiter.ensure_allowed(42).await.is_ok());

        // the counter came back with the allowance, so a single further unchanged result is one of
        // three rather than a fourth, and must not put the node straight back into cooldown
        limiter.record_measurement(42, false).await;
        assert!(limiter.ensure_allowed(42).await.is_ok());
    }

    #[tokio::test]
    async fn one_node_cannot_spend_another_nodes_allowance() {
        let limiter = limiter();
        unchanged_measurements(&limiter, 42, THRESHOLD).await;

        assert!(limiter.ensure_allowed(43).await.is_ok());
    }
}
