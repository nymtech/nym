// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::Client;
use coconut_dkg_common::types::{
    Addr, BlockHeight, DealerDetails, EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey, Epoch,
    EpochId, NodeIndex,
};
use crypto::asymmetric::identity;
use dkg::bte::Ciphertexts;
use dkg::{bte, Dealing, Share};
use futures::lock::Mutex;
use log::debug;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use validator_client::nymd::{AccountId, SigningCosmWasmClient};

mod accessor;

pub(crate) use accessor::StateAccessor;

type IdentityBytes = [u8; identity::PUBLIC_KEY_LENGTH];

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DkgParticipant {
    pub(crate) chain_address: Addr,
    pub(crate) node_index: NodeIndex,
    pub(crate) bte_public_key: bte::PublicKeyWithProof,
    pub(crate) identity: identity::PublicKey,
    pub(crate) remote_address: SocketAddr,
}

impl DkgParticipant {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.chain_address.as_bytes());
        bytes.extend_from_slice(&self.node_index.to_be_bytes());
        bytes.extend_from_slice(&self.bte_public_key.to_bytes());
        bytes.extend_from_slice(&self.identity.to_bytes());
        bytes.extend_from_slice(&self.remote_address.to_string().as_bytes());
        bytes
    }
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

impl DkgParticipant {
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

        Ok(DkgParticipant {
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
    Parsed(DkgParticipant),
}

impl MalformedDealer {
    pub(crate) fn address(&self) -> &Addr {
        match self {
            MalformedDealer::Raw(dealer) => &dealer.address,
            MalformedDealer::Parsed(dealer) => &dealer.chain_address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedDealing {
    epoch_id: u32,
    dealing: Box<Dealing>,
    signature: identity::Signature,
}

pub(crate) struct DealerRegistration {
    pub(crate) identity: EncodedEd25519PublicKey,
    pub(crate) bte_key: EncodedBTEPublicKeyWithProof,
    pub(crate) owner_signature: String,
    pub(crate) network_address: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DkgState {
    inner_state: Arc<Mutex<DkgStateInner>>,
    keys: Arc<Keys>,
}

// we don't want to serialize/deserialize those as they are treated differently
#[derive(Debug)]
struct Keys {
    identity: identity::KeyPair,
    bte_decryption_key: bte::DecryptionKey,
    bte_public_key: bte::PublicKeyWithProof,
}

fn empty_ciphertexts() -> Ciphertexts {
    // this is super temporary and never meant to be used in the long run
    Ciphertexts {
        rr: Default::default(),
        ss: Default::default(),
        zz: Default::default(),
        ciphertext_chunks: vec![],
    }
}

// we want to avoid every storing the actual share in plain on the disk, so only the encrypted version
// is potentially stored and the actual share is decrypted when actually needed (i.e. during aggregation)
// however, if we don't have to serialize data to the disk and everything is kept in memory, there's no
// point in not using what we already have
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StateShare {
    #[serde(skip)]
    share: Option<Share>,

    // the reason for the skip is that I want the code to compile and I haven't yet implemented serde for Ciphertexts : )
    #[serde(skip, default = "empty_ciphertexts")]
    encrypted_share: Ciphertexts,
}

impl StateShare {
    pub(crate) fn new(share: Option<Share>, encrypted_share: Ciphertexts) -> Self {
        StateShare {
            share,
            encrypted_share,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DkgStateInner {
    submitted_keys: bool,
    submitted_commitment: bool,
    submitted_verification_keys: bool,
    network_address: SocketAddr,
    assigned_index: NodeIndex,

    last_seen_height: BlockHeight,
    current_epoch: Epoch,

    expected_epoch_dealing_digests: HashMap<IdentityBytes, [u8; 32]>,

    self_share: Option<StateShare>,

    // we need to keep track of all bad dealers as well so that we wouldn't attempt to complain about them
    // repeatedly
    bad_dealers: HashMap<Addr, MalformedDealer>,
    current_epoch_dealers: HashMap<IdentityBytes, DkgParticipant>,
    verified_epoch_dealings: HashMap<IdentityBytes, ReceivedDealing>,
    unconfirmed_dealings: HashMap<IdentityBytes, ReceivedDealing>,
}

impl DkgState {
    // this should only ever be called once, during init
    pub(crate) async fn initialise_fresh<C>(
        nyxd_client: &Client<C>,
        network_address: SocketAddr,
        identity: identity::KeyPair,
        bte_decryption_key: bte::DecryptionKey,
        bte_public_key: bte::PublicKeyWithProof,
    ) -> Result<Self, DkgError>
    where
        C: SigningCosmWasmClient + Send + Sync,
    {
        debug!("attempting to initialise fresh dkg state");

        let current_epoch = nyxd_client.get_dkg_epoch().await?;

        // TODO: IF we didn't load the state from the file, grab all other data from the contract while
        // we're at it, like dealers, dealing commitments, etc.

        Ok(DkgState {
            inner_state: Arc::new(Mutex::new(DkgStateInner {
                submitted_keys: false,
                submitted_commitment: false,
                submitted_verification_keys: false,
                network_address,
                assigned_index: 0,
                last_seen_height: 0,
                current_epoch,
                expected_epoch_dealing_digests: HashMap::new(),
                self_share: None,
                bad_dealers: HashMap::new(),
                current_epoch_dealers: HashMap::new(),
                verified_epoch_dealings: HashMap::new(),
                unconfirmed_dealings: HashMap::new(),
            })),
            keys: Arc::new(Keys {
                identity,
                bte_decryption_key,
                bte_public_key,
            }),
        })
    }

    pub(crate) async fn load_from_file(&self) {
        todo!()
    }

    // some save/load action here
    pub(crate) async fn save_to_file(&self) {
        todo!()
    }

    // TODO: obviously this would need to get changed in the future in order to account for having to generate MULTIPLE dealings
    pub(crate) async fn generate_dealing(&self) {
        //
    }

    pub(crate) async fn ordered_receivers(&self) -> BTreeMap<NodeIndex, DkgParticipant> {
        self.inner_state
            .lock()
            .await
            .current_epoch_dealers
            .values()
            .map(|participant| (participant.node_index, participant.clone()))
            .collect()
    }

    pub(crate) async fn network_address(&self) -> SocketAddr {
        self.inner_state.lock().await.network_address
    }

    pub(crate) async fn assigned_index(&self) -> NodeIndex {
        self.inner_state.lock().await.assigned_index
    }

    pub(crate) async fn post_key_submission(&self, assigned_index: NodeIndex) {
        let mut guard = self.inner_state.lock().await;
        guard.submitted_keys = true;
        guard.assigned_index = assigned_index;
    }

    pub(crate) async fn post_dealing_submission(&self, share: Option<StateShare>) {
        let mut guard = self.inner_state.lock().await;
        guard.submitted_commitment = true;
        guard.self_share = share;
    }

    pub(crate) async fn is_dealers_remote_address(&self, remote: SocketAddr) -> (bool, Epoch) {
        let guard = self.inner_state.lock().await;
        let epoch = guard.current_epoch;
        let dealers = &guard.current_epoch_dealers;

        (
            dealers
                .values()
                .any(|dealer| dealer.remote_address.ip() == remote.ip()),
            epoch,
        )
    }

    pub(crate) async fn has_submitted_keys(&self) -> bool {
        self.inner_state.lock().await.submitted_keys
    }

    pub(crate) async fn current_epoch(&self) -> Epoch {
        self.inner_state.lock().await.current_epoch
    }

    pub(crate) async fn get_verified_dealing(
        &self,
        dealer: identity::PublicKey,
    ) -> Option<ReceivedDealing> {
        self.inner_state
            .lock()
            .await
            .verified_epoch_dealings
            .get(&dealer.to_bytes())
            .cloned()
    }

    pub(crate) async fn get_known_dealers(&self) -> HashMap<IdentityBytes, DkgParticipant> {
        self.inner_state.lock().await.current_epoch_dealers.clone()
    }

    pub(crate) async fn get_malformed_dealers(&self) -> HashMap<Addr, MalformedDealer> {
        self.inner_state.lock().await.bad_dealers.clone()
    }

    pub(crate) async fn update_last_seen_height(&self, new_last_seen: BlockHeight) {
        self.inner_state.lock().await.last_seen_height = new_last_seen;
    }

    pub(crate) async fn try_add_new_dealer(&self, dealer: DkgParticipant) {
        // TODO: perhaps we should panic or something instead since this should have never occurred in the first place?
        if let Some(old_dealer) = self
            .inner_state
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
        // TODO: perhaps we should panic or something instead since this should have never occurred in the first place?
        if let Some(old_dealer) = self
            .inner_state
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
        let mut guard = self.inner_state.lock().await;

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

    pub(crate) fn sign(&self, message: &[u8]) -> identity::Signature {
        self.keys.identity.private_key().sign(message)
    }

    pub(crate) async fn prepare_dealer_registration(
        &self,
        chain_address: AccountId,
    ) -> DealerRegistration {
        let bte_key = bs58::encode(&self.keys.bte_public_key.to_bytes()).into_string();
        let network_address = self.network_address().await.to_string();

        // chain_address || host || bte_keys
        let mut plaintext = chain_address.to_string();
        plaintext.push_str(&network_address);
        plaintext.push_str(&bte_key);

        let owner_signature = self.sign(plaintext.as_bytes()).to_base58_string();

        DealerRegistration {
            identity: self.keys.identity.public_key().to_base58_string(),
            bte_key,
            owner_signature,
            network_address,
        }
    }

    pub(crate) fn public_bte_key(&self) -> &bte::PublicKey {
        self.keys.bte_public_key.public_key()
    }

    pub(crate) async fn sign_dealing(
        &self,
        epoch_id: EpochId,
        dealing_bytes: &[u8],
    ) -> identity::Signature {
        let mut bytes = Vec::with_capacity(dealing_bytes.len() + std::mem::size_of::<EpochId>());
        bytes.extend_from_slice(&epoch_id.to_be_bytes());
        bytes.extend_from_slice(dealing_bytes);
        self.sign(&bytes)
    }
}
