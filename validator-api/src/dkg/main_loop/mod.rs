// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::dkg::events::DispatcherSender;
use crate::dkg::main_loop::dealing_commitment::{hash_receivers, CommittableEpochDealing};
use crate::dkg::networking::message::OffchainDkgMessage;
use crate::dkg::networking::sender::Broadcaster;
use crate::dkg::smart_contract::publisher::Publisher;
use crate::dkg::smart_contract::watcher;
use crate::dkg::smart_contract::watcher::{CommitmentChange, DealerChange, EventType};
use crate::dkg::state::{DkgParticipant, DkgState, Malformation, MalformedDealer, StateShare};
use coconut_dkg_common::types::{Addr, BlockHeight, DealerDetails, Epoch, EpochId};
use contracts_common::commitment::{Committable, ContractSafeCommitment, MessageCommitment};
use dkg::bte::encrypt_shares;
use dkg::{Dealing, Params};
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, info, trace};
use rand::RngCore;
use std::net::SocketAddr;
use validator_client::nymd::SigningCosmWasmClient;

pub(crate) mod dealing_commitment;

// essentially events originating from the contract watcher could only drive our state forward
// (TODO: is it actually true? I guess we'll find out soon enough)
pub(crate) type ContractEventsReceiver = mpsc::UnboundedReceiver<watcher::Event>;
pub(crate) type ContractEventsSender = mpsc::UnboundedSender<watcher::Event>;

// it is driven by events received from the contract watcher
pub(crate) struct ProcessingLoop<C, R> {
    dkg_params: dkg::Params,

    rng: R,
    // technically we could have used `StateAccessor` here as well, but I really want to keep
    // `StateAccessor` pure from anything that would have to modify `dkg_state`
    dkg_state: DkgState,
    dispatcher_sender: DispatcherSender,
    contract_events_receiver: ContractEventsReceiver,

    contract_publisher: Publisher<C>,
}

impl<C, R: RngCore> ProcessingLoop<C, R>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    pub(crate) fn new(
        rng: R,
        dkg_state: DkgState,
        dispatcher_sender: DispatcherSender,
        contract_events_receiver: ContractEventsReceiver,
        contract_publisher: Publisher<C>,
    ) -> Self {
        ProcessingLoop {
            dkg_params: Params::default(),
            rng,
            dkg_state,
            dispatcher_sender,
            contract_events_receiver,
            contract_publisher,
        }
    }

    fn random_request_id(&mut self) -> u64 {
        self.rng.next_u64()
    }

    async fn broadcast(&self, msg: OffchainDkgMessage, addresses: Vec<SocketAddr>) {
        todo!()
    }

    async fn send_to(&self, msg: OffchainDkgMessage, address: SocketAddr) {
        todo!()
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

    async fn broadcast_dealing(
        &mut self,
        epoch_id: EpochId,
        dealing: Dealing,
        addresses: Vec<SocketAddr>,
    ) -> Result<(), DkgError> {
        let dealing_bytes = dealing.to_bytes();
        let signature = self.dkg_state.sign_dealing(epoch_id, &dealing_bytes).await;
        let request_id = self.random_request_id();

        let msg =
            OffchainDkgMessage::new_dealing_message(request_id, epoch_id, dealing_bytes, signature);

        info!(
            "broadcasting dealing for epoch {} to {} other parties",
            epoch_id,
            addresses.len()
        );
        trace!("parties getting the dealing: {:?}", addresses);

        Broadcaster::new(addresses)
            .broadcast_with_feedback(msg)
            .await
    }

    async fn produce_and_share_dealing(&mut self, epoch: Epoch)
    where
        R: RngCore,
    {
        let self_index = self.dkg_state.assigned_index().await;
        let self_host = self.dkg_state.network_address().await;
        let receivers = self.dkg_state.ordered_receivers().await;
        let receivers_digest = hash_receivers(&receivers);

        let dkg_receivers = receivers
            .iter()
            .map(|(k, v)| (*k, *v.bte_public_key.public_key()))
            .collect();

        // make sure we don't include ourselves (also note: at this point the addresses dont have to be ordered)
        let remote_hosts = receivers
            .values()
            .map(|receiver| receiver.remote_address)
            .filter(|addr| addr != &self_host)
            .collect::<Vec<_>>();

        let threshold = epoch.system_threshold;
        let dkg_epoch = dkg::Epoch::new(epoch.id);

        // for now completely ignore the idea of resharing
        let (dealing, self_share) = Dealing::create(
            &mut self.rng,
            &self.dkg_params,
            self_index,
            threshold,
            dkg_epoch,
            &dkg_receivers,
            None,
        );

        let dealing_bytes = dealing.to_bytes();
        let committable = CommittableEpochDealing::new(
            epoch.id,
            epoch.system_threshold,
            &dealing_bytes,
            &receivers_digest,
        );
        let commitment = committable.produce_commitment();

        // first publish commitment to the chain and then broadcast it to other parties
        if let Err(err) = self
            .contract_publisher
            .submit_dealing_commitment(epoch.id, commitment.contract_safe_commitment())
            .await
        {
            error!("failed to submit dealing commitment to the chain - {}", err);
            // TODO: should we exit the process at this point? This seem to be a rather critical problem
            // as we cannot participate in DKG without that
            return;
        }

        // TODO: should we be broadcasting in separate task to not block the loop for processing other messages?
        if let Err(err) = self
            .broadcast_dealing(epoch.id, dealing, remote_hosts)
            .await
        {
            error!("failed to broadcast our dealing to other parties - {}", err);
            // TODO: again, should we exit here?
            return;
        }

        // TODO: think how to handle it => we want to keep it until we receive all dealings to
        // derive the shared key, but we really want to avoid storing it on disk in plain
        let state_share = self_share.map(|self_share| {
            let (ciphertext, _) = encrypt_shares(
                &[(&self_share, self.dkg_state.public_bte_key())],
                dkg_epoch,
                &self.dkg_params,
                &mut self.rng,
            );
            StateShare::new(Some(self_share), ciphertext)
        });

        self.dkg_state.post_dealing_submission(state_share).await
    }

    async fn deal_with_new_dealer(&self, dealer: DkgParticipant) {
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
        match DkgParticipant::try_parse_from_raw(&dealer_details) {
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

        let chain_address = self.contract_publisher.get_address().await;
        let registration = self
            .dkg_state
            .prepare_dealer_registration(chain_address)
            .await;

        match self
            .contract_publisher
            .register_dealer(
                registration.identity,
                registration.bte_key,
                registration.owner_signature,
                registration.network_address,
            )
            .await
        {
            Err(err) => error!("failed to register our dealer - {}", err),
            Ok(node_index) => {
                info!(
                    "registered our dealer for this DKG round and got assigned index: {}",
                    node_index
                );
                self.dkg_state.post_key_submission(node_index).await
            }
        }
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
    }

    async fn process_commitments_changes(
        &self,
        changes: Vec<CommitmentChange>,
        height: BlockHeight,
    ) {
        debug!(
            "processing known commitments change event with {} changes at height {}",
            changes.len(),
            height
        );
        for change in changes {
            match change {
                CommitmentChange::Addition {
                    address,
                    commitment,
                } => info!("here we would add known commitment"),
                CommitmentChange::Removal { address } => {
                    info!("here we would remove known commitment")
                }
                CommitmentChange::Update {
                    address,
                    commitment,
                } => info!("here we would update known commitment"),
            }
        }
    }

    async fn process_event(&mut self, event: watcher::Event) {
        match event.event_type {
            EventType::NewKeySubmission => self.process_new_key_submission(event.height).await,
            EventType::DealerSetChange { changes } => {
                self.process_dealer_changes(changes, event.height).await
            }
            EventType::NewDealingCommitment { epoch } => {
                self.produce_and_share_dealing(epoch).await
            }
            EventType::NoChange => {
                trace!("no change in the contract, going to only update the last seen height");
            }
            EventType::KnownCommitmentsChange { changes } => {
                self.process_commitments_changes(changes, event.height)
                    .await
            }
        }
        self.dkg_state.update_last_seen_height(event.height).await
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
