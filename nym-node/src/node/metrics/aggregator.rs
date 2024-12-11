// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{HandlerWrapper, MetricsHandler, RegistrableHandler};
use futures::StreamExt;
use nym_node_metrics::events::{
    events_channels, MetricEventsReceiver, MetricEventsSender, MetricsEvent,
};
use std::any;
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{interval_at, Instant};
use tracing::{debug, error, trace, warn};

pub(crate) struct MetricsAggregator {
    // possible issue: this has to be low enough so that frequent handlers would be called sufficiently
    // (if handler doesn't need an update, its internal methods won't be called so it's not going to be wasteful)
    handlers_update_interval: Duration,

    registered_handlers: HashMap<TypeId, Box<dyn RegistrableHandler>>,
    // registered_handlers: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>>,
    event_sender: MetricEventsSender,
    event_receiver: MetricEventsReceiver,
    shutdown: nym_task::TaskClient,
}

impl MetricsAggregator {
    pub fn new(handlers_update_interval: Duration, shutdown: nym_task::TaskClient) -> Self {
        let (event_sender, event_receiver) = events_channels();

        MetricsAggregator {
            handlers_update_interval,
            registered_handlers: Default::default(),
            event_sender,
            event_receiver,
            shutdown,
        }
    }

    pub fn sender(&self) -> MetricEventsSender {
        self.event_sender.clone()
    }

    pub fn register_handler<H>(&mut self, handler: H, update_interval: Duration)
    where
        H: MetricsHandler,
    {
        let events_name = any::type_name::<H::Events>();
        let handler_name = any::type_name::<H>();

        debug!("registering handler '{handler_name}' for events of type '{events_name}'");

        let type_id = TypeId::of::<H::Events>();
        if self.registered_handlers.contains_key(&type_id) {
            panic!("duplicate handler for '{events_name}' (id: {type_id:?})",)
        };

        self.registered_handlers.insert(
            type_id,
            Box::new(HandlerWrapper::new(update_interval, handler)),
        );
    }

    async fn periodic_handlers_update(&mut self) {
        for handler in self.registered_handlers.values_mut() {
            handler.on_update().await;
        }
    }

    async fn handle_metrics_event<T: 'static>(&mut self, event: T) {
        let Some(handler) = self.registered_handlers.get_mut(&TypeId::of::<T>()) else {
            let name = any::type_name::<T>();

            warn!("no registered handler for events of type {name}");
            return;
        };

        #[allow(clippy::expect_used)]
        let handler: &mut HandlerWrapper<T> = handler
            .deref_mut()
            .as_any_mut()
            .downcast_mut()
            .expect("handler downcasting failure");

        handler.handle_event(event).await;
    }

    async fn handle_event(&mut self, event: MetricsEvent) {
        match event {
            MetricsEvent::GatewayClientSession(client_session) => {
                self.handle_metrics_event(client_session).await
            }
        }
    }

    async fn on_start(&mut self) {
        for handler in self.registered_handlers.values_mut() {
            handler.on_start().await;
        }
    }

    pub async fn run(&mut self) {
        self.on_start().await;

        let start = Instant::now() + self.handlers_update_interval;
        let mut update_interval = interval_at(start, self.handlers_update_interval);

        let mut processed = 0;
        trace!("starting MetricsAggregator");
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    debug!("MetricsAggregator: Received shutdown");
                    break;
                }
                _ = update_interval.tick() => {
                    self.periodic_handlers_update().await;
                }
                new_event = self.event_receiver.next() => {
                    // this one is impossible to ever panic - the struct itself contains a sender
                    // and hence it can't happen that ALL senders are dropped
                    #[allow(clippy::unwrap_used)]
                    self.handle_event(new_event.unwrap()).await;
                    if processed % 1000 == 0 {
                        let queue_len = self.event_sender.len();
                        match queue_len {
                            n if n > 200 => error!("there are currently {n} pending events waiting to get processed!"),
                            n if n > 50 => warn!("there are currently {n} pending events waiting to get processed"),
                            n => trace!("there are currently {n} pending events waiting to get processed"),
                        }
                    }
                    processed += 1;
                }

            }
        }
        trace!("MetricsAggregator: Exiting");
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
