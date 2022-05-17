// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::events::{DispatcherSender, Event};
use crate::dkg::smart_contract::watcher;
use crate::dkg::state::{
    DkgParticipant, DkgState, IdentityBytes, MalformedDealer, ReceivedDealing,
};
use coconut_dkg_common::types::{Addr, BlockHeight, Epoch};
use crypto::asymmetric::identity;
use std::collections::HashMap;
use std::net::SocketAddr;

// essentially some intermediary that allows either pushing events to the dispatcher or operating
// directly on the dkg state
// note: it should get renamed, but at the time of writing it, I couldn't come up with anything...
#[derive(Debug, Clone)]
pub(crate) struct StateAccessor {
    dkg_state: DkgState,
    dispatcher_sender: DispatcherSender,
}

impl StateAccessor {
    pub(crate) fn new(dkg_state: DkgState, dispatcher_sender: DispatcherSender) -> Self {
        StateAccessor {
            dkg_state,
            dispatcher_sender,
        }
    }

    pub(crate) async fn push_event(&self, event: Event) {
        if let Err(err) = self.dispatcher_sender.unbounded_send(event) {
            log::error!("Our event dispatcher failed to receive {} event - it has presumably crashed. Shutting down the API after saving DKG state", err.into_inner());
            self.dkg_state.save_to_file().await;
            std::process::exit(1);
        }
    }

    pub(crate) async fn push_contract_change_event(&self, event: watcher::Event) {
        self.push_event(Event::new_contract_change_event(event))
            .await
    }

    pub(crate) async fn push_new_key_submission_event(&self, block_height: BlockHeight) {
        self.push_event(Event::new_contract_change_event(watcher::Event::new(
            block_height,
            watcher::EventType::NewKeySubmission,
        )))
        .await
    }

    pub(crate) async fn has_submitted_keys(&self) -> bool {
        self.dkg_state.has_submitted_keys().await
    }

    pub(crate) async fn current_epoch(&self) -> Epoch {
        self.dkg_state.current_epoch().await
    }

    pub(crate) async fn get_verified_dealing(
        &self,
        dealer: identity::PublicKey,
    ) -> Option<ReceivedDealing> {
        self.dkg_state.get_verified_dealing(dealer).await
    }

    pub(crate) async fn is_dealers_remote_address(&self, remote: SocketAddr) -> (bool, Epoch) {
        self.dkg_state.is_dealers_remote_address(remote).await
    }

    pub(crate) async fn get_known_dealers(&self) -> HashMap<IdentityBytes, DkgParticipant> {
        self.dkg_state.get_known_dealers().await
    }

    pub(crate) async fn get_malformed_dealers(&self) -> HashMap<Addr, MalformedDealer> {
        self.dkg_state.get_malformed_dealers().await
    }
}
