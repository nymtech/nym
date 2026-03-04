// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::state::SharedLpClientControlState;
use nym_lp::peer_config::LpReceiverIndex;
use nym_metrics::{add_histogram_obs, inc};
use nym_registration_common::dvpn::{
    LpDvpnRegistrationFinalisation, LpDvpnRegistrationInitialRequest,
    LpDvpnRegistrationRequestMessage, LpDvpnRegistrationRequestMessageContent,
};
use nym_registration_common::mixnet::LpMixnetRegistrationRequestMessage;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationRequestData, LpRegistrationResponse, RegistrationMode,
    RegistrationStatus,
};
use tracing::*;

// Histogram buckets for LP registration duration tracking
// Registration includes credential verification, DB operations, and potentially WireGuard peer setup
// Expected durations: 100ms - 5s for normal operations, up to 30s for slow DB or network issues
const LP_REGISTRATION_DURATION_BUCKETS: &[f64] = &[
    0.1,  // 100ms
    0.25, // 250ms
    0.5,  // 500ms
    1.0,  // 1s
    2.5,  // 2.5s
    5.0,  // 5s
    10.0, // 10s
    30.0, // 30s
];

impl SharedLpClientControlState {
    async fn process_dvpn_initial_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpDvpnRegistrationInitialRequest,
    ) -> LpRegistrationResponse {
        let Some(registrator) = self.peer_registrator.as_ref() else {
            return LpRegistrationResponse::error(
                "dVPN via LP is not enabled on this node",
                RegistrationMode::Dvpn,
            );
        };

        registrator
            .on_initial_lp_request(request, sender)
            .await
            .unwrap_or_else(|err| {
                LpRegistrationResponse::error(
                    format!("LP registration has failed: {err}"),
                    RegistrationMode::Dvpn,
                )
            })
    }

    async fn process_dvpn_registration_finalisation(
        &self,
        sender: LpReceiverIndex,
        request: LpDvpnRegistrationFinalisation,
    ) -> LpRegistrationResponse {
        let Some(registrator) = self.peer_registrator.as_ref() else {
            return LpRegistrationResponse::error(
                "dVPN via LP is not enabled on this node",
                RegistrationMode::Dvpn,
            );
        };

        registrator
            .on_final_lp_request(request, sender)
            .await
            .unwrap_or_else(|err| {
                LpRegistrationResponse::error(
                    format!("LP registration has failed: {err}"),
                    RegistrationMode::Dvpn,
                )
            })
    }

    async fn process_dvpn_registration(
        &self,
        sender: LpReceiverIndex,
        request: Box<LpDvpnRegistrationRequestMessage>,
    ) -> LpRegistrationResponse {
        // Track dVPN registration attempts
        inc!("lp_registration_dvpn_attempts");

        match request.content {
            LpDvpnRegistrationRequestMessageContent::InitialRequest(req) => {
                self.process_dvpn_initial_registration(sender, req).await
            }
            LpDvpnRegistrationRequestMessageContent::Finalisation(req) => {
                self.process_dvpn_registration_finalisation(sender, req)
                    .await
            }
        }
    }

    async fn process_mixnet_registration(
        &self,
        request: LpMixnetRegistrationRequestMessage,
    ) -> LpRegistrationResponse {
        let _ = request;
        LpRegistrationResponse::error(
            "mixnet registration is not yet supported",
            RegistrationMode::Mixnet,
        )
    }

    /// Process an LP registration request
    pub async fn process_registration(
        &self,
        sender: LpReceiverIndex,
        request: LpRegistrationRequest,
    ) -> LpRegistrationResponse {
        let registration_start = std::time::Instant::now();

        // Track total registration attempts
        inc!("lp_registration_attempts_total");

        // 1. Validate timestamp for replay protection
        if !request.validate_timestamp(30) {
            warn!("LP registration failed: timestamp too old or too far in future");
            inc!("lp_registration_failed_timestamp");
            return LpRegistrationResponse::error("invalid timestamp", request.mode());
        }

        // 2. Process based on mode
        let result = match request.registration_data {
            LpRegistrationRequestData::Dvpn { data } => {
                self.process_dvpn_registration(sender, data).await
            }
            LpRegistrationRequestData::Mixnet { data } => {
                self.process_mixnet_registration(data).await
            }
        };

        // Track registration duration
        let duration = registration_start.elapsed().as_secs_f64();
        add_histogram_obs!(
            "lp_registration_duration_seconds",
            duration,
            LP_REGISTRATION_DURATION_BUCKETS
        );

        // Track overall success/failure
        match result.status {
            RegistrationStatus::Completed => {
                inc!("lp_registration_success_total");
            }
            RegistrationStatus::Failed => {
                inc!("lp_registration_failed_total");
            }
            RegistrationStatus::PendingMoreData => {
                inc!("lp_registration_pending_more_data");
            }
        }

        result
    }
}
