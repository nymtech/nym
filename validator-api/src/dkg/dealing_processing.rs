// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::message::NewDealingMessage;
use crate::dkg::state::DkgState;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, error};

// Once the DKG epoch begins, all parties will begin exchanging dealings with each other.
// We really don't want to be processing all of them in parallel since we would starve other
// parts of the process of CPU. Hence we put them in a queue and process each of them one by one.
pub(crate) type DealingReceiver = mpsc::UnboundedReceiver<NewDealingMessage>;
pub(crate) type DealingSender = mpsc::UnboundedSender<NewDealingMessage>;

pub(crate) struct Processor {
    // TODO: should it hold the actual dkg_state or rather the stateaccessor and emit events regarding processed dealing?
    dkg_state: DkgState,
    receiver: DealingReceiver,
}
impl Processor {
    pub(crate) fn new(dkg_state: DkgState, receiver: DealingReceiver) -> Self {
        Processor {
            dkg_state,
            receiver,
        }
    }

    async fn process_dealing(&self, dealing: NewDealingMessage) {
        todo!()
    }

    pub(crate) async fn run(&mut self) {
        debug!("starting Dealing Processor");

        while let Some(dealing) = self.receiver.next().await {
            self.process_dealing(dealing).await
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("Dealing Processor has stopped receiving events! The process is in an undefined state. Shutting down...");
        std::process::exit(1);
    }
}
