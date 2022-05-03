// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::dealing_processing::DealingSender;
use crate::dkg::events::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use log::error;

pub(crate) type DispatcherSender = mpsc::UnboundedSender<Event>;
pub(crate) type DispatcherReceiver = mpsc::UnboundedReceiver<Event>;

pub(crate) struct Dispatcher {
    event_receiver: DispatcherReceiver,

    dealing_processor: DealingSender,
}

impl Dispatcher {
    fn handle_event(&self, event: Event) {
        match event {
            Event::NewDealing(new_dealing_request) => self
                .dealing_processor
                .unbounded_send(new_dealing_request)
                .expect("failed to forward new dealing message"),
            _ => todo!(),
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(new_event) = self.event_receiver.next().await {
            self.handle_event(new_event)
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("")
    }
}
