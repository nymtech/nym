// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::smart_contract::publisher::Publisher;
use crate::dkg::smart_contract::watcher;
use crate::dkg::state::StateAccessor;
use coconut_dkg_common::types::DealerDetails;
use futures::channel::mpsc;
use futures::StreamExt;

// essentially events originating from the contract watcher could only drive our state forward
// (TODO: is it actually true? I guess we'll find out soon enough)
pub(crate) type ContractEventsReceiver = mpsc::UnboundedReceiver<watcher::Event>;
pub(crate) type ContractEventsSender = mpsc::UnboundedSender<watcher::Event>;

// it is driven by events received from the contract watcher
pub(crate) struct ProcessingLoop<C> {
    state_accessor: StateAccessor,
    contract_events_receiver: ContractEventsReceiver,

    contract_publisher: Publisher<C>,
    // network_sender: Sender
}

impl<C> ProcessingLoop<C> {
    fn verify_dealer(&self, contract_dealer_details: &DealerDetails) {
        //
    }

    async fn process_event(&self, event: watcher::Event) {
        todo!()
    }

    pub(crate) async fn run(&mut self) {
        while let Some(event) = self.contract_events_receiver.next().await {
            self.process_event(event).await
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("DKG Processing Loop has stopped receiving events! The process is in an undefined state. Shutting down...");
        std::process::exit(1);
    }
}
