// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_gateway::node::PersistentStatsStorage;
use nym_gateway_stats_storage::error::StatsStorageError;
use nym_gateway_stats_storage::models::{TicketType, ToSessionType};
use nym_node_metrics::entry::{ActiveSession, ClientSessions, FinishedSession};
use nym_node_metrics::events::GatewaySessionEvent;
use nym_node_metrics::NymNodeMetrics;
use nym_sphinx_types::DestinationAddressBytes;
use time::{Date, Duration, OffsetDateTime};
use tracing::error;
use tracing::log::trace;

pub(crate) struct GatewaySessionStatsHandler {
    storage: PersistentStatsStorage,
    current_day: Date,

    metrics: NymNodeMetrics,
}

impl GatewaySessionStatsHandler {
    pub(crate) fn new(metrics: NymNodeMetrics, storage: PersistentStatsStorage) -> Self {
        GatewaySessionStatsHandler {
            storage,
            current_day: OffsetDateTime::now_utc().date(),
            metrics,
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
            .update_active_session_type(client, ticket_type.to_session_type())
            .await?;
        Ok(())
    }

    async fn handle_session_delete(
        &mut self,
        client: DestinationAddressBytes,
    ) -> Result<(), StatsStorageError> {
        self.storage.delete_active_session(client).await?;
        self.storage.delete_unique_user(client).await?;
        Ok(())
    }

    async fn handle_session_event(
        &mut self,
        event: GatewaySessionEvent,
    ) -> Result<(), StatsStorageError> {
        match event {
            GatewaySessionEvent::SessionStart { start_time, client } => {
                self.handle_session_start(start_time, client).await
            }

            GatewaySessionEvent::SessionStop { stop_time, client } => {
                self.handle_session_stop(stop_time, client).await
            }

            GatewaySessionEvent::EcashTicket {
                ticket_type,
                client,
            } => self.handle_ecash_ticket(ticket_type, client).await,

            // As long as delete is sent before stop, everything should work as expected
            GatewaySessionEvent::SessionDelete { client } => {
                self.handle_session_delete(client).await
            }
        }
    }

    async fn maybe_update_shared_state(
        &mut self,
        update_time: OffsetDateTime,
    ) -> Result<(), StatsStorageError> {
        let update_date = update_time.date();
        if update_date != self.current_day {
            self.update_shared_stats(self.current_day).await?;
            self.delete_old_stats(self.current_day - Duration::DAY)
                .await?;
            self.reset_stats(update_date).await?;
            self.current_day = update_date;
        }
        Ok(())
    }

    // update shared state once a day has passed, with data from the previous day
    async fn update_shared_stats(&mut self, stats_date: Date) -> Result<(), StatsStorageError> {
        let finished_sessions = self.storage.get_finished_sessions(stats_date).await?;
        let unique_users = self.storage.get_unique_users(stats_date).await?;
        let session_started = self.storage.get_started_sessions_count(stats_date).await? as u32;

        let new_sessions = ClientSessions::new(
            stats_date,
            unique_users,
            session_started,
            finished_sessions.into_iter().map(Into::into).collect(),
        );
        self.metrics
            .entry
            .update_client_sessions(new_sessions)
            .await;

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

    async fn cleanup(&mut self) -> Result<(), StatsStorageError> {
        let yesterday = OffsetDateTime::now_utc().date() - Duration::DAY;
        //publish yesterday's data if any
        self.update_shared_stats(yesterday).await?;
        //store "active" sessions as duration 0
        for active_session in self.storage.get_all_active_sessions().await? {
            self.storage
                .insert_finished_session(
                    self.current_day,
                    FinishedSession::new(Default::default(), active_session.typ),
                )
                .await?
        }
        //cleanup active sessions
        self.storage.cleanup_active_sessions().await?;

        //delete old entries
        self.delete_old_stats(yesterday - Duration::DAY).await?;
        Ok(())
    }
}

#[async_trait]
impl OnStartMetricsHandler for GatewaySessionStatsHandler {
    async fn on_start(&mut self) {
        if let Err(err) = self.cleanup().await {
            error!("failed to cleanup gateway session stats handler: {err}");
        }
    }
}

#[async_trait]
impl OnUpdateMetricsHandler for GatewaySessionStatsHandler {
    async fn on_update(&mut self) {
        let now = OffsetDateTime::now_utc();
        if let Err(err) = self.maybe_update_shared_state(now).await {
            error!("failed to update session stats: {err}");
        }
    }
}

#[async_trait]
impl MetricsHandler for GatewaySessionStatsHandler {
    type Events = GatewaySessionEvent;

    async fn handle_event(&mut self, event: Self::Events) {
        trace!("event: {event:?}");
        if let Err(err) = self.handle_session_event(event).await {
            error!("failed to handle client session event '{event:?}': {err}")
        }
    }
}
