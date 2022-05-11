// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Addr, BlockHeight, DealerDetails, Epoch, NodeIndex};
use crypto::asymmetric::identity;
use dkg::{bte, Dealing};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

mod accessor;

use crate::dkg::error::DkgError;
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

// TODO: move it elsewhere and propagate it to the contract
pub enum MalformedDealer {
    MalformedEd25519PublicKey,
    MalformedBTEPublicKey,
    InvalidBTEPublicKey,
    InvalidHostInformation,
}

impl Dealer {
    pub(crate) fn try_parse_from_raw(
        contract_value: DealerDetails,
    ) -> Result<Self, MalformedDealer> {
        // this should be impossible as the contract must have used this key for signature verification
        let identity = identity::PublicKey::from_base58_string(contract_value.ed25519_public_key)
            .map_err(|_| MalformedDealer::MalformedEd25519PublicKey)?;

        let bte_public_key = bs58::decode(contract_value.bte_public_key_with_proof)
            .into_vec()
            .map(|bytes| bte::PublicKeyWithProof::try_from_bytes(&bytes))
            .map_err(|_| MalformedDealer::MalformedBTEPublicKey)?
            .map_err(|_| MalformedDealer::MalformedBTEPublicKey)?;

        if !bte_public_key.verify() {
            return Err(MalformedDealer::InvalidBTEPublicKey);
        }

        let parsed_host = contract_value
            .host
            .parse()
            .map_err(|_| MalformedDealer::InvalidHostInformation)?;

        Ok(Dealer {
            chain_address: contract_value.address,
            node_index: contract_value.assigned_index,
            bte_public_key,
            identity,
            remote_address: parsed_host,
        })
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

    // we need to keep track of all bad dealers as well so that we wouldn't attempt to compalaint about them
    // repeatedly
    bad_dealers: HashSet<Addr>,
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

    pub(crate) async fn get_malformed_dealers(&self) -> HashSet<Addr> {
        self.inner.lock().await.bad_dealers.clone()
    }
}
