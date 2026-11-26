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
