// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::{DbPool, Storage};
use crate::ticketbook_manager::state::TicketbookManagerState;
use nym_task::ShutdownToken;

pub(crate) mod state;
pub(crate) mod storage;

pub struct TicketbookManager {
    state: TicketbookManagerState,
}

impl TicketbookManager {
    pub(crate) fn new(storage: Storage) -> Self {
        todo!()
    }

    pub async fn run(&self, shutdown_token: ShutdownToken) {
        loop {
            //
        }
        todo!()
    }
}
