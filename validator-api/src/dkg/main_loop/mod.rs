// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::smart_contract::watcher;
use crate::dkg::state::{DkgState, StateAccessor};
use futures::channel::mpsc;
use futures::StreamExt;

// essentially events originating from the contract watcher could only drive our state forward
// (TODO: is it actually true? I guess we'll find out soon enough)
pub(crate) type ContractEventsReceiver = mpsc::UnboundedReceiver<watcher::Event>;
pub(crate) type ContractEventsSender = mpsc::UnboundedSender<watcher::Event>;

// it is driven by events received from the contract watcher
pub(crate) struct ProcessingLoop {
    state_accessor: StateAccessor,
    contract_events_receiver: ContractEventsReceiver,
}

impl ProcessingLoop {
    async fn process_event(&self, event: watcher::Event) {
        todo!()
    }

    pub(crate) async fn run(&mut self) {
        while let Some(event) = self.contract_events_receiver.next().await {
            self.process_event(event).await
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("")
    }
}
