// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::dealing_processing::DealingSender;
use crate::dkg::events::Event;
use crate::dkg::main_loop::ContractEventsSender;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, error, trace};
use std::fmt::Display;

pub(crate) type DispatcherSender = mpsc::UnboundedSender<Event>;
pub(crate) type DispatcherReceiver = mpsc::UnboundedReceiver<Event>;

pub(crate) struct Dispatcher {
    event_receiver: DispatcherReceiver,

    dealing_processor: DealingSender,
    contract_event_sender: ContractEventsSender,
}

impl Dispatcher {
    pub(crate) fn new(
        event_receiver: DispatcherReceiver,
        dealing_processor: DealingSender,
        contract_event_sender: ContractEventsSender,
    ) -> Self {
        Dispatcher {
            event_receiver,
            dealing_processor,
            contract_event_sender,
        }
    }

    // we require `T` to be explicitly `Display` for the purposes of providing better error messages
    // before crashing
    fn forward_event<T: Display>(&self, channel: &mpsc::UnboundedSender<T>, event_item: T) {
        if let Err(err) = channel.unbounded_send(event_item) {
            log::error!("Our event dispatcher failed to forward {} event - the receiver has presumably crashed. Shutting down the API...", err.into_inner());
            std::process::exit(1);
        }
    }

    fn handle_event(&self, event: Event) {
        match event {
            Event::NewDealing(new_dealing_request) => {
                trace!("received and forwarding NewDealing Event");
                self.forward_event(&self.dealing_processor, new_dealing_request)
            }
            Event::DkgContractChange(watcher_event) => {
                trace!("received and forwarding DkgContractChange Event");
                self.forward_event(&self.contract_event_sender, watcher_event)
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("starting Dispatcher");
        while let Some(new_event) = self.event_receiver.next().await {
            self.handle_event(new_event)
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("Event Dispatcher has stopped receiving events! The process is in an undefined state. Shutting down...");
        std::process::exit(1);
    }
}
