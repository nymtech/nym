// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use super::SharedNodeStats;

// TODO: question: should this data still be logged to the console or should we perhaps remove it
// since we have the http endpoint now?
pub struct PacketStatsConsoleLogger {
    logging_delay: Duration,
    stats: SharedNodeStats,
}

impl PacketStatsConsoleLogger {
    pub fn new(logging_delay: Duration, stats: SharedNodeStats) -> Self {
        PacketStatsConsoleLogger {
            logging_delay,
            stats,
        }
    }

    async fn log_running_stats(&mut self) {
        let stats = self.stats.read().await;

        // it's super unlikely this will ever fail, but anything involving time is super weird
        // so let's just guard against it
        if let Ok(time_difference) = stats.update_time.duration_since(stats.previous_update_time) {
            // we honestly don't care if it was 30.000828427s or 30.002461449s, 30s is enough
            let difference_secs = time_difference.as_secs();

            info!(
                "Since startup mixed {} packets! ({} in last {} seconds)",
                stats.packets_sent_since_startup.values().sum::<u64>(),
                stats.packets_sent_since_last_update.values().sum::<u64>(),
                difference_secs,
            );
            if !stats.packets_explicitly_dropped_since_startup.is_empty() {
                info!(
                    "Since startup dropped {} packets! ({} in last {} seconds)",
                    stats
                        .packets_explicitly_dropped_since_startup
                        .values()
                        .sum::<u64>(),
                    stats
                        .packets_explicitly_dropped_since_last_update
                        .values()
                        .sum::<u64>(),
                    difference_secs,
                );
            }

            debug!(
                "Since startup received {} packets ({} in last {} seconds)",
                stats.packets_received_since_startup,
                stats.packets_received_since_last_update,
                difference_secs,
            );
            trace!(
                "Since startup sent packets to the following: \n{:#?} \n And in last {} seconds: {:#?})",
                stats.packets_sent_since_startup,
                difference_secs,
                stats.packets_sent_since_last_update
            );
        } else {
            info!(
                "Since startup mixed {} packets!",
                stats.packets_sent_since_startup.values().sum::<u64>(),
            );
            if !stats.packets_explicitly_dropped_since_startup.is_empty() {
                info!(
                    "Since startup dropped {} packets!",
                    stats
                        .packets_explicitly_dropped_since_startup
                        .values()
                        .sum::<u64>(),
                );
            }

            debug!(
                "Since startup received {} packets",
                stats.packets_received_since_startup
            );
            trace!(
                "Since startup sent packets to the following: \n{:#?}",
                stats.packets_sent_since_startup
            );
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::time::sleep(self.logging_delay).await;
            self.log_running_stats().await;
        }
    }
}
