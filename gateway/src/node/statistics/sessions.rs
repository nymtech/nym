// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use nym_gateway_stats_storage::PersistentStatsStorage;
use nym_gateway_stats_storage::{error::StatsStorageError, models::ActiveSession};
use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_sphinx::DestinationAddressBytes;
use time::{Date, Duration, OffsetDateTime};

use nym_statistics_common::events::SessionEvent;

pub(crate) struct SessionStatsHandler {
    storage: PersistentStatsStorage,
    last_update_day: Date,

    shared_session_stats: SharedSessionStats,
    sessions_started: u32,
}

impl SessionStatsHandler {
    pub fn new(shared_session_stats: SharedSessionStats, storage: PersistentStatsStorage) -> Self {
        SessionStatsHandler {
            storage,
            last_update_day: OffsetDateTime::now_utc().date(),
            shared_session_stats,
            sessions_started: 0,
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
        self.sessions_started += 1;
        self.storage
            .insert_unique_user(self.last_update_day, client.as_base58_string())
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
                    .insert_finished_session(self.last_update_day, finished_session)
                    .await?;
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
        //cleanup active sessions
        self.storage.cleanup_active_sessions().await?;
        //reset stats
        self.reset_stats(yesterday).await?;
        Ok(())
    }

    //update shared state once a day has passed, with data from the previous day
    async fn publish_stats(&mut self, stats_date: Date) -> Result<(), StatsStorageError> {
        let finished_sessions = self.storage.get_finished_sessions(stats_date).await?;
        let user_count = self.storage.get_unique_users(stats_date).await?;
        {
            let mut shared_state = self.shared_session_stats.write().await;
            shared_state.update_time = stats_date;
            shared_state.unique_active_users = user_count as u32;
            shared_state.session_started = self.sessions_started;
            shared_state.sessions = finished_sessions.iter().map(|s| s.serialize()).collect();
        }

        Ok(())
    }
    pub(crate) async fn maybe_update_shared_state(
        &mut self,
        update_time: OffsetDateTime,
    ) -> Result<(), StatsStorageError> {
        let update_date = update_time.date();
        if update_date != self.last_update_day {
            self.publish_stats(self.last_update_day).await?;
            self.reset_stats(self.last_update_day).await?;
        }
        Ok(())
    }

    async fn reset_stats(&mut self, reset_day: Date) -> Result<(), StatsStorageError> {
        self.last_update_day = reset_day;

        //active users reset
        let new_active_users = self.storage.get_active_users().await?;
        self.storage
            .delete_unique_users(reset_day - Duration::DAY)
            .await?;
        for user in new_active_users {
            self.storage.insert_unique_user(reset_day, user).await?;
        }

        //finished session reset
        self.storage
            .delete_finished_sessions(reset_day - Duration::DAY)
            .await?;
        self.sessions_started = 0;
        Ok(())
    }
}
