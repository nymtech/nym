// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::mpsc;
use nym_sphinx::DestinationAddressBytes;
use time::OffsetDateTime;

pub type StatsEventSender = mpsc::UnboundedSender<StatsEvent>;
pub type StatsEventReceiver = mpsc::UnboundedReceiver<StatsEvent>;
pub enum StatsEvent {
    SessionStatsEvent(SessionEvent),
}

impl StatsEvent {
    pub fn new_session_start(client: DestinationAddressBytes) -> StatsEvent {
        StatsEvent::SessionStatsEvent(SessionEvent::SessionStart {
            start_time: OffsetDateTime::now_utc(),
            client,
        })
    }

    pub fn new_session_stop(client: DestinationAddressBytes) -> StatsEvent {
        StatsEvent::SessionStatsEvent(SessionEvent::SessionStop {
            stop_time: OffsetDateTime::now_utc(),
            client,
        })
    }
}

pub enum SessionEvent {
    SessionStart {
        start_time: OffsetDateTime,
        client: DestinationAddressBytes,
    },
    SessionStop {
        stop_time: OffsetDateTime,
        client: DestinationAddressBytes,
    },
}
