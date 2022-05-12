// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use std::time::Duration;
use tokio::time;

const STATISTICS_INTERVAL: Duration = Duration::from_secs(60);

pub type TimerReceiver = mpsc::Receiver<()>;

pub struct Timer {
    interval: Duration,
    stats_sender: mpsc::Sender<()>,
}

impl Timer {
    pub fn new() -> (Self, TimerReceiver) {
        let (stats_sender, stats_receiver) = mpsc::channel::<()>(1);
        (
            Timer {
                interval: STATISTICS_INTERVAL,
                stats_sender,
            },
            stats_receiver,
        )
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub async fn run(&mut self) {
        let mut interval = time::interval(self.interval);
        loop {
            interval.tick().await;
            let _ = self.stats_sender.try_send(());
        }
    }
}
