// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::error::RequestError;
use nym_validator_client::nyxd::nym_performance_contract_common::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;

/// The most recent request served for each node, so a captured one cannot be submitted twice.
///
/// Without this, a single intercepted request could be replayed until the target node's burst
/// allowance was gone, which is a cheap denial of service to mount against a competitor.
///
/// Monotonic rather than a set of signatures already seen, matching the `declared_at` rule the
/// contract applies to self-declarations: one timestamp per node answers the same question as a
/// set of every signature ever served, without growing with the number of requests or needing to
/// be expired. The cost is that a node cannot have two requests accepted within the same second,
/// which is not a rate a legitimate one asks at.
#[derive(Clone)]
pub(crate) struct ReplayGuard {
    validity_window: Duration,

    // one small entry per node that has asked, and only a bonded node whose signature has already
    // verified ever reaches this, so it is bounded by the bonded set rather than by traffic
    last_accepted: Arc<Mutex<HashMap<NodeId, OffsetDateTime>>>,
}

impl ReplayGuard {
    pub(crate) fn new(validity_window: Duration) -> ReplayGuard {
        ReplayGuard {
            validity_window,
            last_accepted: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Accept a request only if it is fresh and newer than the last one served for that node.
    ///
    /// Both halves are needed and neither suffices alone: the window alone would let a captured
    /// request be replayed freely until it expired, and monotonicity alone would accept a request
    /// signed years ago as long as nothing newer had arrived.
    ///
    /// MUST be called only once the signature has been verified. A request recorded before that
    /// would let anybody advance a node's timestamp with a forgery and lock the real node out for
    /// as long as they cared to keep doing it.
    pub(crate) async fn accept_once(
        &self,
        node_id: NodeId,
        signed_at: OffsetDateTime,
    ) -> Result<(), RequestError> {
        let now = OffsetDateTime::now_utc();

        // rejected in both directions. one from the future is not merely useless: it would let a
        // node mint requests now and hold them until their windows opened, turning the burst limit
        // into something it could spend at a time of its choosing
        if (now - signed_at).unsigned_abs() > self.validity_window {
            return Err(RequestError::unauthorised(
                "the request timestamp is outside the validity window",
            ));
        }

        let mut last_accepted = self.last_accepted.lock().await;
        if let Some(previous) = last_accepted.get(&node_id) {
            if signed_at <= *previous {
                return Err(RequestError::unauthorised(
                    "a request at least as recent has already been served for this node",
                ));
            }
        }
        last_accepted.insert(node_id, signed_at);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_secs(30);

    fn guard() -> ReplayGuard {
        ReplayGuard::new(WINDOW)
    }

    #[tokio::test]
    async fn a_fresh_request_is_accepted() {
        assert!(
            guard()
                .accept_once(42, OffsetDateTime::now_utc())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn the_same_request_is_not_served_twice() {
        let guard = guard();
        let signed_at = OffsetDateTime::now_utc();

        assert!(guard.accept_once(42, signed_at).await.is_ok());
        assert!(guard.accept_once(42, signed_at).await.is_err());
    }

    #[tokio::test]
    async fn an_older_request_is_rejected_after_a_newer_one() {
        // a captured request stays valid for the rest of its window, so without this its replay
        // would be indistinguishable from a first delivery
        let guard = guard();
        let now = OffsetDateTime::now_utc();

        assert!(guard.accept_once(42, now).await.is_ok());
        assert!(
            guard
                .accept_once(42, now - time::Duration::seconds(5))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_strictly_newer_request_is_accepted() {
        let guard = guard();
        let now = OffsetDateTime::now_utc();

        assert!(
            guard
                .accept_once(42, now - time::Duration::seconds(5))
                .await
                .is_ok()
        );
        assert!(guard.accept_once(42, now).await.is_ok());
    }

    #[tokio::test]
    async fn a_stale_request_is_rejected() {
        let stale = OffsetDateTime::now_utc() - WINDOW - time::Duration::seconds(1);

        assert!(guard().accept_once(42, stale).await.is_err());
    }

    #[tokio::test]
    async fn a_request_from_the_future_is_rejected() {
        // otherwise a node could sign a batch of requests now and release them one window at a
        // time, spending its burst allowance whenever it suited rather than when it asked
        let ahead = OffsetDateTime::now_utc() + WINDOW + time::Duration::seconds(1);

        assert!(guard().accept_once(42, ahead).await.is_err());
    }

    #[tokio::test]
    async fn one_node_cannot_lock_out_another() {
        let guard = guard();
        let now = OffsetDateTime::now_utc();

        assert!(guard.accept_once(42, now).await.is_ok());
        assert!(
            guard
                .accept_once(43, now - time::Duration::seconds(5))
                .await
                .is_ok()
        );
    }
}
