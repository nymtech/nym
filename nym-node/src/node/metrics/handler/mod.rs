// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use std::any;
use std::any::Any;
use std::time::Duration;
use tokio::time::Instant;
use tracing::trace;

pub(crate) mod client_sessions;
pub(crate) mod global_prometheus_updater;
pub(crate) mod legacy_packet_data;
pub(crate) mod mixnet_data_cleaner;
pub(crate) mod prometheus_events_handler;

pub(crate) trait RegistrableHandler:
    Downcast + OnStartMetricsHandler + OnUpdateMetricsHandler + Send + Sync + 'static
{
}

impl<T> RegistrableHandler for T where
    T: Downcast + OnStartMetricsHandler + OnUpdateMetricsHandler + Send + Sync + 'static
{
}

pub trait Downcast {
    #[allow(dead_code)]
    fn as_any(&'_ self) -> &'_ dyn Any
    where
        Self: 'static;

    fn as_any_mut(&'_ mut self) -> &mut dyn Any
    where
        Self: 'static;
}

impl<T> Downcast for T {
    fn as_any(&'_ self) -> &'_ dyn Any
    where
        Self: 'static,
    {
        self
    }

    fn as_any_mut(&'_ mut self) -> &'_ mut dyn Any
    where
        Self: 'static,
    {
        self
    }
}

#[async_trait]
pub(crate) trait MetricsHandler: RegistrableHandler {
    type Events;

    async fn handle_event(&mut self, event: Self::Events);
}

#[async_trait]
pub(crate) trait OnStartMetricsHandler {
    async fn on_start(&mut self) {}
}

#[async_trait]
pub(crate) trait OnUpdateMetricsHandler {
    async fn on_update(&mut self) {}
}

pub(crate) struct HandlerWrapper<T> {
    handler: Box<dyn MetricsHandler<Events = T>>,
    update_interval: Option<Duration>,
    last_updated: Instant,
}

impl<T> HandlerWrapper<T> {
    pub fn new<U>(update_interval: impl Into<Option<Duration>>, handler: U) -> Self
    where
        U: MetricsHandler<Events = T>,
    {
        HandlerWrapper {
            handler: Box::new(handler),
            update_interval: update_interval.into(),
            last_updated: Instant::now(),
        }
    }
}

impl<T> HandlerWrapper<T>
where
    T: 'static,
{
    pub(crate) async fn handle_event(&mut self, event: T) {
        self.handler.handle_event(event).await
    }
}

#[async_trait]
impl<T> OnStartMetricsHandler for HandlerWrapper<T> {
    async fn on_start(&mut self) {
        let name = any::type_name::<T>();
        trace!("on start for handler for events of type {name}");

        self.handler.on_start().await;
    }
}

#[async_trait]
impl<T> OnUpdateMetricsHandler for HandlerWrapper<T> {
    async fn on_update(&mut self) {
        let Some(update_interval) = self.update_interval else {
            return;
        };

        let name = any::type_name::<T>();
        trace!("on update for handler for events of type {name}");

        let elapsed = self.last_updated.elapsed();
        if elapsed < update_interval {
            trace!("too soon for updates");
            return;
        }

        self.handler.on_update().await;
        self.last_updated = Instant::now();
    }
}
