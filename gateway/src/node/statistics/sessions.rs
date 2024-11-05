// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use nym_gateway_stats_storage::models::FinishedSession;
use nym_gateway_stats_storage::PersistentStatsStorage;
use nym_gateway_stats_storage::{error::StatsStorageError, models::ActiveSession};
use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_sphinx::DestinationAddressBytes;
use sha2::{Digest, Sha256};
use time::{Date, Duration, OffsetDateTime};

use nym_statistics_common::gateways::SessionEvent;

pub(crate) struct SessionStatsHandler {
    storage: PersistentStatsStorage,
    current_day: Date,

    shared_session_stats: SharedSessionStats,
}

impl SessionStatsHandler {
    pub fn new(shared_session_stats: SharedSessionStats, storage: PersistentStatsStorage) -> Self {
        SessionStatsHandler {
            storage,
            current_day: OffsetDateTime::now_utc().date(),
            shared_session_stats,
        }
    }

    pub(crate) async fn handle_event(
        &mut self,
        event: SessionEvent,
    ) -> Result<(), StatsStorageError> {
        match event {
            SessionEvent::SessionStart { start_time, client } => {
                self.handle_session_start(start_time, client).await
            }

            SessionEvent::SessionStop { stop_time, client } => {
                self.handle_session_stop(stop_time, client).await
            }

            SessionEvent::EcashTicket {
                ticket_type,
                client,
            } => self.handle_ecash_ticket(ticket_type, client).await,
        }
    }
    async fn handle_session_start(
        &mut self,
        start_time: OffsetDateTime,
        client: DestinationAddressBytes,
    ) -> Result<(), StatsStorageError> {
        self.storage
            .insert_unique_user(self.current_day, client.as_base58_string())
            .await?;
        self.storage
            .insert_active_session(client, ActiveSession::new(start_time))
            .await?;
        Ok(())
    }

    async fn handle_session_stop(
        &mut self,
        stop_time: OffsetDateTime,
        client: DestinationAddressBytes,
    ) -> Result<(), StatsStorageError> {
        if let Some(session) = self.storage.get_active_session(client).await? {
            if let Some(finished_session) = session.end_at(stop_time) {
                self.storage
                    .insert_finished_session(self.current_day, finished_session)
                    .await?;
                self.storage.delete_active_session(client).await?;
            }
        }
        Ok(())
    }

    async fn handle_ecash_ticket(
        &mut self,
        ticket_type: TicketType,
        client: DestinationAddressBytes,
    ) -> Result<(), StatsStorageError> {
        self.storage
            .update_active_session_type(client, ticket_type.into())
            .await?;
        Ok(())
    }

    pub(crate) async fn on_start(&mut self) -> Result<(), StatsStorageError> {
        let yesterday = OffsetDateTime::now_utc().date() - Duration::DAY;
        //publish yesterday's data if any
        self.publish_stats(yesterday).await?;
        //store "active" sessions as duration 0
        for active_session in self.storage.get_all_active_sessions().await? {
            self.storage
                .insert_finished_session(
                    self.current_day,
                    FinishedSession {
                        duration: Duration::ZERO,
                        typ: active_session.typ,
                    },
                )
                .await?
        }
        //cleanup active sessions
        self.storage.cleanup_active_sessions().await?;

        //delete old entries
        self.delete_old_stats(yesterday - Duration::DAY).await?;
        Ok(())
    }

    //update shared state once a day has passed, with data from the previous day
    async fn publish_stats(&mut self, stats_date: Date) -> Result<(), StatsStorageError> {
        let finished_sessions = self.storage.get_finished_sessions(stats_date).await?;
        let unique_users = self.storage.get_unique_users(stats_date).await?;
        let unique_users_hash = unique_users
            .into_iter()
            .map(|address| format!("{:x}", Sha256::digest(address)))
            .collect::<Vec<_>>();
        let session_started = self.storage.get_started_sessions_count(stats_date).await? as u32;
        {
            let mut shared_state = self.shared_session_stats.write().await;
            shared_state.update_time = stats_date;
            shared_state.unique_active_users_count = unique_users_hash.len() as u32;
            shared_state.unique_active_users_hashes = unique_users_hash;
            shared_state.session_started = session_started;
            shared_state.sessions = finished_sessions.iter().map(|s| s.serialize()).collect();
        }

        Ok(())
    }
    pub(crate) async fn maybe_update_shared_state(
        &mut self,
        update_time: OffsetDateTime,
    ) -> Result<(), StatsStorageError> {
        let update_date = update_time.date();
        if update_date != self.current_day {
            self.publish_stats(self.current_day).await?;
            self.delete_old_stats(self.current_day - Duration::DAY)
                .await?;
            self.reset_stats(update_date).await?;
            self.current_day = update_date;
        }
        Ok(())
    }

    async fn reset_stats(&mut self, reset_day: Date) -> Result<(), StatsStorageError> {
        //active users reset
        let new_active_users = self.storage.get_active_users().await?;
        for user in new_active_users {
            self.storage.insert_unique_user(reset_day, user).await?;
        }

        Ok(())
    }

    async fn delete_old_stats(&mut self, delete_before: Date) -> Result<(), StatsStorageError> {
        self.storage.delete_finished_sessions(delete_before).await?;
        self.storage.delete_unique_users(delete_before).await?;
        Ok(())
    }
}
