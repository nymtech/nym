// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::v1::error::ApiError;
use crate::http::state::work::storage_failure;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::FiveHopMixnetSession;
use nym_network_monitor_orchestrator_requests::models::{
    FiveHopMixnetSessionData, TestMessageData,
};
use nym_network_monitor_route_records::test_message::SessionTestMessages;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;

/// Every stored session naming `mixnet_epoch`, as test messages with the verdict stored for each.
pub(super) async fn five_hop_mixnet_sessions_of_epoch(
    storage: &NetworkMonitorStorage,
    mixnet_epoch: EpochId,
) -> Result<Vec<FiveHopMixnetSessionData>, ApiError> {
    let sessions = storage
        .get_five_hop_mixnet_sessions_of_epoch(mixnet_epoch)
        .await
        .map_err(|err| storage_failure("get_five_hop_mixnet_sessions_of_epoch", err))?;

    // a session the scoring could not read is left out here too
    Ok(sessions
        .iter()
        .filter_map(five_hop_mixnet_session_data)
        .collect())
}

/// A stored five-hop mixnet session in its public shape; `None` when its batch no longer reads.
fn five_hop_mixnet_session_data(
    session: &FiveHopMixnetSession,
) -> Option<FiveHopMixnetSessionData> {
    let test_messages = session.test_messages()?;

    Some(FiveHopMixnetSessionData {
        run_id: session.run_id,
        // stored from a `NodeId`, so it fits
        gateway_node_id: session.gateway_node_id as NodeId,
        kept: session.kept,
        test_messages: test_message_data(test_messages),
    })
}

/// A session's test messages in their public shape.
fn test_message_data(session: SessionTestMessages) -> Vec<TestMessageData> {
    session
        .test_messages
        .into_iter()
        .map(|message| TestMessageData {
            sent_at_ms: message.sent_at_ms,
            path: message.path,
            came_back: message.came_back,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{five_hop_mixnet_batch, five_hop_mixnet_session};
    use std::time::Duration;

    const EPOCH: EpochId = 6;

    /// A session's exported test messages: send time in ms, path, came back.
    fn exported_messages(session: &FiveHopMixnetSessionData) -> Vec<(u64, [u32; 3], bool)> {
        session
            .test_messages
            .iter()
            .map(|message| (message.sent_at_ms, message.path, message.came_back))
            .collect()
    }

    // Sessions come in the order their runs started and a run's by gateway node id, each with the
    // verdict stored at submission, and a session's test messages in the order sent. Each run's
    // higher gateway is submitted first, so only the export's ordering puts 100 first.
    #[tokio::test]
    async fn an_epochs_sessions_are_exported_in_order_with_their_verdicts() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let kept = five_hop_mixnet_batch(101, EPOCH, 1000..1003, 1);
        let first_run = storage
            .store_submitted_five_hop_mixnet_run(
                &[&kept],
                &[&five_hop_mixnet_batch(100, EPOCH, 1000..1003, 3)],
            )
            .await;
        let second_run = storage
            .store_submitted_five_hop_mixnet_run(
                &[&five_hop_mixnet_batch(101, EPOCH, 2000..2003, 0)],
                &[&five_hop_mixnet_batch(100, EPOCH, 2000..2003, 3)],
            )
            .await;

        let exported = five_hop_mixnet_sessions_of_epoch(&storage, EPOCH)
            .await
            .unwrap();

        let sessions: Vec<(i64, u32, bool)> = exported
            .iter()
            .map(|session| (session.run_id, session.gateway_node_id, session.kept))
            .collect();
        assert_eq!(
            sessions,
            vec![
                (first_run, 100, false),
                (first_run, 101, true),
                (second_run, 100, false),
                (second_run, 101, true),
            ]
        );

        let expected: Vec<(u64, [u32; 3], bool)> = SessionTestMessages::from_batch(&kept)
            .test_messages
            .into_iter()
            .map(|message| (message.sent_at_ms, message.path, message.came_back))
            .collect();
        assert_eq!(exported_messages(&exported[1]), expected);
    }

    // An epoch with no stored session is an empty list, even beside a stored epoch.
    #[tokio::test]
    async fn an_epoch_without_sessions_is_an_empty_list() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let batch = five_hop_mixnet_batch(100, EPOCH - 1, 0..3, 0);
        storage
            .store_submitted_five_hop_mixnet_run(&[&batch], &[])
            .await;

        let exported = five_hop_mixnet_sessions_of_epoch(&storage, EPOCH)
            .await
            .unwrap();

        assert!(exported.is_empty());
    }

    // The scoring leaves out a kept session whose stored batch no longer reads, so the export does
    // too: the epoch is still answered, with the sessions that do read.
    #[tokio::test]
    async fn a_session_whose_batch_no_longer_reads_is_left_out() {
        let storage = NetworkMonitorStorage::in_memory().await;
        let run_id = storage
            .start_five_hop_mixnet_run(Duration::ZERO, Duration::from_secs(60))
            .await
            .unwrap()
            .unwrap();
        let intact = five_hop_mixnet_batch(100, EPOCH, 0..3, 0);
        let mut damaged =
            five_hop_mixnet_session(run_id, &five_hop_mixnet_batch(101, EPOCH, 0..3, 0), true);
        damaged.batch.pop();
        storage
            .submit_five_hop_mixnet_run(
                run_id,
                &[five_hop_mixnet_session(run_id, &intact, true), damaged],
            )
            .await
            .unwrap();

        let exported = five_hop_mixnet_sessions_of_epoch(&storage, EPOCH)
            .await
            .unwrap();

        let gateways: Vec<u32> = exported
            .iter()
            .map(|session| session.gateway_node_id)
            .collect();
        assert_eq!(gateways, vec![100]);
    }
}
