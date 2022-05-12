// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Addr, BlockHeight, DealerDetails, Epoch, NodeIndex};
use crypto::asymmetric::identity;
use dkg::{bte, Dealing};
use futures::lock::Mutex;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

mod accessor;

pub(crate) use accessor::StateAccessor;

type IdentityBytes = [u8; identity::PUBLIC_KEY_LENGTH];

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Dealer {
    pub(crate) chain_address: Addr,
    pub(crate) node_index: NodeIndex,
    pub(crate) bte_public_key: bte::PublicKeyWithProof,
    pub(crate) identity: identity::PublicKey,
    pub(crate) remote_address: SocketAddr,
}

impl Dealer {
    pub(crate) fn map_key(&self) -> IdentityBytes {
        self.identity.to_bytes()
    }
}

// TODO: move it elsewhere and propagate it to the contract
#[derive(Debug)]
pub enum Malformation {
    MalformedEd25519PublicKey,
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    InvalidHostInformation,
}

impl Dealer {
    pub(crate) fn try_parse_from_raw(contract_value: &DealerDetails) -> Result<Self, Malformation> {
        // this should be impossible as the contract must have used this key for signature verification
        let identity = identity::PublicKey::from_base58_string(&contract_value.ed25519_public_key)
            .map_err(|_| Malformation::MalformedEd25519PublicKey)?;

        let bte_public_key = bs58::decode(&contract_value.bte_public_key_with_proof)
            .into_vec()
            .map(|bytes| bte::PublicKeyWithProof::try_from_bytes(&bytes))
            .map_err(|_| Malformation::MalformedBTEPublicKey)?
            .map_err(|_| Malformation::MalformedBTEPublicKey)?;

        if !bte_public_key.verify() {
            return Err(Malformation::InvalidBTEPublicKey);
        }

        let parsed_host = contract_value
            .host
            .parse()
            .map_err(|_| Malformation::InvalidHostInformation)?;

        Ok(Dealer {
            chain_address: contract_value.address.clone(),
            node_index: contract_value.assigned_index,
            bte_public_key,
            identity,
            remote_address: parsed_host,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum MalformedDealer {
    Raw(DealerDetails),
    Parsed(Dealer),
}

impl MalformedDealer {
    pub(crate) fn address(&self) -> &Addr {
        match self {
            MalformedDealer::Raw(dealer) => &dealer.address,
            MalformedDealer::Parsed(dealer) => &dealer.chain_address,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DkgState {
    inner: Arc<Mutex<DkgStateInner>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedDealing {
    epoch_id: u32,
    dealing: Box<Dealing>,
    signature: identity::Signature,
}

#[derive(Debug, Serialize, Deserialize)]
struct DkgStateInner {
    last_seen_height: BlockHeight,
    bte_decryption_key: bte::DecryptionKey,
    signing_key: identity::PublicKey,

    current_epoch: Epoch,

    expected_epoch_dealing_digests: HashMap<IdentityBytes, [u8; 32]>,

    // we need to keep track of all bad dealers as well so that we wouldn't attempt to complaint about them
    // repeatedly
    bad_dealers: HashMap<Addr, MalformedDealer>,
    current_epoch_dealers: HashMap<IdentityBytes, Dealer>,
    verified_epoch_dealings: HashMap<IdentityBytes, ReceivedDealing>,
    unconfirmed_dealings: HashMap<IdentityBytes, ReceivedDealing>,
}

impl DkgState {
    // some save/load action here
    pub(crate) async fn save(&self) {
        todo!()
    }

    pub(crate) async fn is_dealers_remote_address(&self, remote: SocketAddr) -> (bool, Epoch) {
        let guard = self.inner.lock().await;
        let epoch = guard.current_epoch;
        let dealers = &guard.current_epoch_dealers;

        (
            dealers
                .values()
                .any(|dealer| dealer.remote_address == remote),
            epoch,
        )
    }

    pub(crate) async fn current_epoch(&self) -> Epoch {
        self.inner.lock().await.current_epoch
    }

    pub(crate) async fn get_verified_dealing(
        &self,
        dealer: identity::PublicKey,
    ) -> Option<ReceivedDealing> {
        self.inner
            .lock()
            .await
            .verified_epoch_dealings
            .get(&dealer.to_bytes())
            .cloned()
    }

    pub(crate) async fn get_known_dealers(&self) -> HashMap<IdentityBytes, Dealer> {
        self.inner.lock().await.current_epoch_dealers.clone()
    }

    pub(crate) async fn get_malformed_dealers(&self) -> HashMap<Addr, MalformedDealer> {
        self.inner.lock().await.bad_dealers.clone()
    }

    pub(crate) async fn update_last_seen_height(&self, new_last_seen: BlockHeight) {
        self.inner.lock().await.last_seen_height = new_last_seen;
    }

    pub(crate) async fn try_add_new_dealer(&self, dealer: Dealer) {
        // TODO: perhaps we should panic or something instead since this should have never occured in the first place?
        if let Some(old_dealer) = self
            .inner
            .lock()
            .await
            .current_epoch_dealers
            .insert(dealer.map_key(), dealer)
        {
            error!(
                "We have overwritten {} dealer details",
                old_dealer.chain_address
            )
        }
    }

    pub(crate) async fn try_add_malformed_dealer(&self, dealer_details: MalformedDealer) {
        // TODO: perhaps we should panic or something instead since this should have never occured in the first place?
        if let Some(old_dealer) = self
            .inner
            .lock()
            .await
            .bad_dealers
            .insert(dealer_details.address().clone(), dealer_details)
        {
            error!(
                "We have overwritten {} dealer details",
                old_dealer.address()
            )
        }
    }

    pub(crate) async fn try_remove_dealer(&self, dealer_address: Addr) {
        let mut guard = self.inner.lock().await;

        // dealer is in either bad dealers or known dealers, never both,
        // so if we managed to remove it from the former, we don't need to check the latter
        if guard.bad_dealers.remove(&dealer_address).is_none() {
            // find storage key associated with the entry we want to remove
            let storage_key = guard
                .current_epoch_dealers
                .values()
                .find(|&dealer| dealer.chain_address == dealer_address)
                .map(|dealer| dealer.map_key());

            match storage_key {
                Some(key) => {
                    guard.current_epoch_dealers.remove(&key);
                }
                // this should be impossible as in order to get to this point we must have learned about
                // this dealer existing somewhere in our state!
                None => error!(
                    "We failed to remove {} dealer details as it somehow doesn't exist!",
                    dealer_address
                ),
            }
        }
    }
}
