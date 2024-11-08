// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub type ClientStatsReceiver = tokio::sync::mpsc::UnboundedReceiver<ClientStatsEvent>;

#[derive(Clone)]
pub struct ClientStatsReporter {
    stats_tx: tokio::sync::mpsc::UnboundedSender<ClientStatsEvent>,
}

impl ClientStatsReporter {
    pub fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<ClientStatsEvent>) -> Self {
        Self { stats_tx }
    }

    pub fn report(&self, event: ClientStatsEvent) {
        self.stats_tx.send(event).unwrap_or_else(|err| {
            log::error!("Failed to report client stat event : {:?}", err);
        });
    }
}

pub enum ClientStatsEvent {
    //SW TODO this enum is WIP
}
