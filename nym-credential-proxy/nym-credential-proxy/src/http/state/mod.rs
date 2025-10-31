// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::nyx_upgrade_mode::UpgradeModeState;
use nym_credential_proxy_lib::ticketbook_manager::TicketbookManager;
use nym_credential_proxy_requests::api::v1::ticketbook::models::UpgradeModeResponse;
use std::future::Future;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) mod nyx_upgrade_mode;

#[derive(Clone)]
pub struct ApiState {
    ticketbooks: TicketbookManager,
    upgrade_mode: UpgradeModeState,
}

impl ApiState {
    pub(crate) fn new(ticketbooks: TicketbookManager, upgrade_mode: UpgradeModeState) -> Self {
        Self {
            ticketbooks,
            upgrade_mode,
        }
    }

    pub(crate) fn ticketbooks(&self) -> &TicketbookManager {
        &self.ticketbooks
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.ticketbooks.shutdown_token()
    }

    pub(crate) fn try_spawn_in_background<F>(&self, task: F) -> Option<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.ticketbooks().try_spawn_in_background(task)
    }

    pub(crate) async fn upgrade_mode_response(&self) -> Option<UpgradeModeResponse> {
        let (upgrade_mode_attestation, jwt) = self.upgrade_mode.attestation_with_jwt().await?;
        Some(UpgradeModeResponse {
            upgrade_mode_attestation,
            jwt,
        })
    }
}
