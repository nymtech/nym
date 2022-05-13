// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::events::DispatcherSender;
use crate::dkg::smart_contract::publisher::Publisher;
use crate::dkg::smart_contract::watcher;
use crate::dkg::smart_contract::watcher::{DealerChange, EventType};
use crate::dkg::state::{Dealer, DkgState, Malformation, MalformedDealer};
use coconut_dkg_common::types::{Addr, BlockHeight, DealerDetails};
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, info, trace};

// essentially events originating from the contract watcher could only drive our state forward
// (TODO: is it actually true? I guess we'll find out soon enough)
pub(crate) type ContractEventsReceiver = mpsc::UnboundedReceiver<watcher::Event>;
pub(crate) type ContractEventsSender = mpsc::UnboundedSender<watcher::Event>;

// it is driven by events received from the contract watcher
pub(crate) struct ProcessingLoop<C> {
    // technically we could have used `StateAccessor` here as well, but I really want to keep
    // `StateAccessor` pure from anything that would have to modify `dkg_state`
    dkg_state: DkgState,
    dispatcher_sender: DispatcherSender,
    contract_events_receiver: ContractEventsReceiver,

    contract_publisher: Publisher<C>,
    // network_sender: Sender
}

impl<C> ProcessingLoop<C> {
    pub(crate) fn new(
        dkg_state: DkgState,
        dispatcher_sender: DispatcherSender,
        contract_events_receiver: ContractEventsReceiver,
        contract_publisher: Publisher<C>,
    ) -> Self {
        ProcessingLoop {
            dkg_state,
            dispatcher_sender,
            contract_events_receiver,
            contract_publisher,
        }
    }

    async fn raise_malformed_dealer_complaint(
        &self,
        dealer_address: Addr,
        malformation: Malformation,
    ) {
        info!(
            "here we would be raising complaint about dealer {} being malformed: {:?}",
            dealer_address, malformation
        );
        // todo!()
    }

    async fn deal_with_new_dealer(&self, dealer: Dealer) {
        if dealer.bte_public_key.verify() {
            self.dkg_state.try_add_new_dealer(dealer).await
        } else {
            debug!("received dealer {} failed to prove possession of its BTE key and it will be dealt with accordingly", dealer.chain_address);
            let dealer_address = dealer.chain_address.clone();
            // the dealer failed to provide valid proof of possession
            let malformation = Malformation::InvalidBTEPublicKey;
            self.dkg_state
                .try_add_malformed_dealer(MalformedDealer::Parsed(dealer))
                .await;
            self.raise_malformed_dealer_complaint(dealer_address, malformation)
                .await;
        }
    }

    async fn deal_with_malformed_dealer(
        &self,
        dealer_details: DealerDetails,
        malformation: Malformation,
    ) {
        debug!(
            "received dealer {} is malformed ({:?}) and it will be dealt with accordingly",
            dealer_details.address, malformation
        );
        let dealer_address = dealer_details.address.clone();
        self.dkg_state
            .try_add_malformed_dealer(MalformedDealer::Raw(dealer_details))
            .await;
        self.raise_malformed_dealer_complaint(dealer_address, malformation)
            .await;
    }

    async fn process_new_dealer(&self, dealer_details: DealerDetails) {
        trace!("processing new dealer ({})", dealer_details.address);
        match Dealer::try_parse_from_raw(&dealer_details) {
            Ok(dealer) => self.deal_with_new_dealer(dealer).await,
            Err(malformed_dealer) => {
                self.deal_with_malformed_dealer(dealer_details, malformed_dealer)
                    .await
            }
        }
    }

    async fn process_dealer_removal(&self, dealer_address: Addr) {
        trace!("processing dealer removal ({})", dealer_address);
        self.dkg_state.try_remove_dealer(dealer_address).await
    }

    async fn process_new_key_submission(&self, height: BlockHeight) {
        debug!("attempting to register our own dealer keys for this round of dkg");
        info!(".... but that's not implemented yet....");
    }

    async fn process_dealer_changes(&self, changes: Vec<DealerChange>, height: BlockHeight) {
        debug!(
            "processing dealer set change event with {} changes at height {}",
            changes.len(),
            height
        );
        for change in changes {
            match change {
                DealerChange::Addition { details } => self.process_new_dealer(details).await,
                DealerChange::Removal { address } => self.process_dealer_removal(address).await,
            }
        }
        self.dkg_state.update_last_seen_height(height).await
    }

    async fn process_event(&self, event: watcher::Event) {
        match event.event_type {
            EventType::NewKeySubmission => self.process_new_key_submission(event.height).await,
            EventType::DealerSetChange { changes } => {
                self.process_dealer_changes(changes, event.height).await
            }
            EventType::NewDealingCommitment => todo!(),
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("starting DKG main processing loop");

        while let Some(event) = self.contract_events_receiver.next().await {
            self.process_event(event).await
        }

        // since we have no graceful shutdowns, seeing this error means something bad has happened
        // as all senders got dropped
        error!("DKG Processing Loop has stopped receiving events! The process is in an undefined state. Shutting down...");
        std::process::exit(1);
    }
}
