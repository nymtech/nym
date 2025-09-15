// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credential_proxy_lib::ticketbook_manager::TicketbookManager;

#[derive(Clone)]
pub struct ApiState {
    inner: TicketbookManager,
}

impl From<TicketbookManager> for ApiState {
    fn from(inner: TicketbookManager) -> Self {
        Self { inner }
    }
}

impl ApiState {
    pub(crate) fn inner_state(&self) -> &TicketbookManager {
        &self.inner
    }
}
