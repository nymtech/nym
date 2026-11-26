// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::aggregation::five_hop_liveness::FiveHopMixnetSessionTestMessages;
use crate::http::api::v1::error::ApiError;
use crate::storage::NetworkMonitorStorage;
use crate::storage::models::FiveHopMixnetSession;
use nym_network_monitor_orchestrator_requests::models::{
    RouteSessionData, RouteSessionOutcomeData, TestMessageData,
};
use nym_network_monitor_route_records::test_message::SessionTestMessages;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::EpochId;
use time::OffsetDateTime;
use tracing::error;

/// Every session of each route run with a session naming `mixnet_epoch`, as test messages with the
/// verdict stored for each.
pub(super) async fn five_hop_mixnet_sessions_of_epoch(
    storage: &NetworkMonitorStorage,
    mixnet_epoch: EpochId,
) -> Result<Vec<RouteSessionData>, ApiError> {
    let sessions = match storage
        .get_five_hop_mixnet_sessions_of_epoch(mixnet_epoch)
        .await
    {
        Err(err) => {
            error!("get_five_hop_mixnet_sessions_of_epoch storage failure: {err}");
            return Err(ApiError::StorageFailure);
        }
        Ok(sessions) => sessions,
    };

    sessions.iter().map(route_session_data).collect()
}

/// A stored route session in its public shape.
fn route_session_data(session: &FiveHopMixnetSession) -> Result<RouteSessionData, ApiError> {
    let outcome = match FiveHopMixnetSessionTestMessages::read(session) {
        FiveHopMixnetSessionTestMessages::Error(error) => RouteSessionOutcomeData::Error(error),
        FiveHopMixnetSessionTestMessages::Kept(kept) => {
            RouteSessionOutcomeData::Kept(test_message_data(kept)?)
        }
        FiveHopMixnetSessionTestMessages::Discarded(discarded) => {
            RouteSessionOutcomeData::Discarded(test_message_data(discarded)?)
        }
    };

    Ok(RouteSessionData {
        route_run_id: session.run_id,
        // stored from a `NodeId`, so it fits
        gateway_node_id: session.gateway_node_id as NodeId,
        outcome,
    })
}

/// A session's test messages in their public shape; a send time out of range is corruption.
fn test_message_data(session: SessionTestMessages) -> Result<Vec<TestMessageData>, ApiError> {
    session
        .test_messages
        .into_iter()
        .map(|message| {
            let sent_at_ns = i128::from(message.sent_at_ms) * 1_000_000;
            let sent_at = OffsetDateTime::from_unix_timestamp_nanos(sent_at_ns).map_err(|err| {
                error!("could not build a test message out of a stored batch: {err}");
                ApiError::MalformedStoredData
            })?;

            Ok(TestMessageData {
                sent_at,
                path: message.path,
                came_back: message.came_back,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{
        FIXTURE_SEEN_AT, failed_five_hop_mixnet_session, five_hop_mixnet_batch,
        five_hop_mixnet_session, gateway,
    };

    const EPOCH: EpochId = 6;

    /// Storage with gateways 100 and 101 bonded, to carry route runs.
    async fn storage_with_gateways() -> NetworkMonitorStorage {
        let storage = NetworkMonitorStorage::in_memory().await;
        storage
            .store_refresh(&[gateway(100), gateway(101)], FIXTURE_SEEN_AT)
            .await
            .unwrap();
        storage
    }

    /// The verdict a session is exported with.
    fn verdict(outcome: &RouteSessionOutcomeData) -> &'static str {
        match outcome {
            RouteSessionOutcomeData::Error(_) => "error",
            RouteSessionOutcomeData::Kept(_) => "kept",
            RouteSessionOutcomeData::Discarded(_) => "discarded",
        }
    }

    /// A kept or discarded session's exported test messages: send time in ms, path, came back.
    fn exported_messages(outcome: &RouteSessionOutcomeData) -> Vec<(i128, [u32; 3], bool)> {
        let (RouteSessionOutcomeData::Kept(messages)
        | RouteSessionOutcomeData::Discarded(messages)) = outcome
        else {
            panic!("a failed session has no test messages");
        };
        messages
            .iter()
            .map(|message| {
                let sent_at_ms = message.sent_at.unix_timestamp_nanos() / 1_000_000;
                (sent_at_ms, message.path, message.came_back)
            })
            .collect()
    }

    // Sessions come in the order their runs started and a run's by gateway node id, each with the
    // verdict stored at submission or its error, and a session's test messages in the order sent.
    // Each run's higher gateway is submitted first, so only the export's ordering puts 100 first.
    #[tokio::test]
    async fn an_epochs_sessions_are_exported_in_order_with_their_verdicts() {
        let storage = storage_with_gateways().await;
        let kept = five_hop_mixnet_batch(101, EPOCH, 1000..1003, 1);
        let first_run = storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&kept, true),
                five_hop_mixnet_session(&five_hop_mixnet_batch(100, EPOCH, 1000..1003, 3), false),
            ])
            .await;
        let second_run = storage
            .store_submitted_five_hop_mixnet_run(&[
                five_hop_mixnet_session(&five_hop_mixnet_batch(101, EPOCH, 2000..2003, 0), true),
                failed_five_hop_mixnet_session(100),
            ])
            .await;

        let exported = five_hop_mixnet_sessions_of_epoch(&storage, EPOCH)
            .await
            .unwrap();

        let sessions: Vec<(i64, u32, &str)> = exported
            .iter()
            .map(|session| {
                let verdict = verdict(&session.outcome);
                (session.route_run_id, session.gateway_node_id, verdict)
            })
            .collect();
        assert_eq!(
            sessions,
            vec![
                (first_run, 100, "discarded"),
                (first_run, 101, "kept"),
                (second_run, 100, "error"),
                (second_run, 101, "kept"),
            ]
        );

        let expected: Vec<(i128, [u32; 3], bool)> = SessionTestMessages::from_batch(&kept)
            .test_messages
            .into_iter()
            .map(|message| {
                (
                    i128::from(message.sent_at_ms),
                    message.path,
                    message.came_back,
                )
            })
            .collect();
        assert_eq!(exported_messages(&exported[1].outcome), expected);
    }

    // An epoch with no stored session is an empty list, even beside a stored epoch.
    #[tokio::test]
    async fn an_epoch_without_sessions_is_an_empty_list() {
        let storage = storage_with_gateways().await;
        let batch = five_hop_mixnet_batch(100, EPOCH - 1, 0..3, 0);
        storage
            .store_submitted_five_hop_mixnet_run(&[five_hop_mixnet_session(&batch, true)])
            .await;

        let exported = five_hop_mixnet_sessions_of_epoch(&storage, EPOCH)
            .await
            .unwrap();

        assert!(exported.is_empty());
    }
}
