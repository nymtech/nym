// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::controller::keys::archive_coconut_keypair;
use crate::coconut::dkg::state::dealing_exchange::DealingExchangeState;
use crate::coconut::dkg::state::key_derivation::KeyDerivationState;
use crate::coconut::dkg::state::registration::RegistrationState;
use crate::coconut::error::CoconutError;
use crate::coconut::keys::{KeyPair as CoconutKeyPair, KeyPairWithEpoch};
use cosmwasm_std::Addr;
use log::debug;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::types::{
    DealingIndex, EncodedBTEPublicKeyWithProof, EpochId, EpochState,
};
use nym_crypto::asymmetric::identity;
use nym_dkg::bte::{keys::KeyPair as DkgKeyPair, PublicKey, PublicKeyWithProof};
use nym_dkg::{bte, Dealing, NodeIndex, RecoveredVerificationKeys, Threshold};
use nym_validator_client::nyxd::{tx, Hash};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_helpers::{bte_pk_serde, generated_dealings_old, vks_serde};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use tokio::sync::RwLockReadGuard;
use url::Url;
use crate::coconut::dkg::state::key_finalization::FinalizationState;
use crate::coconut::dkg::state::key_validation::ValidationState;

mod dealing_exchange;
mod key_derivation;
mod key_finalization;
mod key_validation;
mod registration;
mod serde_helpers;

#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) enum ParticipantState {
    Invalid(ComplaintReason),
    VerifiedKey(#[serde(with = "bte_pk_serde")] PublicKeyWithProof),
}

impl ParticipantState {
    pub fn is_valid(&self) -> bool {
        matches!(self, ParticipantState::VerifiedKey(..))
    }

    pub fn public_key(&self) -> Option<bte::PublicKey> {
        match self {
            ParticipantState::Invalid(_) => None,
            ParticipantState::VerifiedKey(key_with_proof) => Some(*key_with_proof.public_key()),
        }
    }

    fn from_raw_encoded_key(raw: EncodedBTEPublicKeyWithProof) -> Self {
        // TODO: include more error information
        let Ok(bytes) = bs58::decode(raw).into_vec() else {
            return ParticipantState::Invalid(ComplaintReason::MalformedBTEPublicKey);
        };

        let Ok(key) = PublicKeyWithProof::try_from_bytes(&bytes) else {
            return ParticipantState::Invalid(ComplaintReason::MalformedBTEPublicKey);
        };

        if !key.verify() {
            return ParticipantState::Invalid(ComplaintReason::InvalidBTEPublicKey);
        }

        ParticipantState::VerifiedKey(key)
    }
}

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) struct DkgParticipant {
    pub(crate) address: Addr,
    pub(crate) assigned_index: NodeIndex,
    pub(crate) state: ParticipantState,
}

impl DkgParticipant {
    #[cfg(test)]
    pub(crate) fn unwrap_key(&self) -> PublicKeyWithProof {
        if let ParticipantState::VerifiedKey(key) = &self.state {
            return key.clone();
        }
        panic!("no key")
    }
}

impl From<DealerDetails> for DkgParticipant {
    fn from(dealer: DealerDetails) -> Self {
        DkgParticipant {
            address: dealer.address,
            state: ParticipantState::from_raw_encoded_key(dealer.bte_public_key_with_proof),
            assigned_index: dealer.assigned_index,
        }
    }
}

#[async_trait]
pub(crate) trait ConsistentState {
    fn node_index_value(&self) -> Result<NodeIndex, CoconutError>;
    fn receiver_index_value(&self) -> Result<usize, CoconutError>;
    fn threshold(&self) -> Result<Threshold, CoconutError>;
    async fn coconut_keypair_is_some(&self) -> Result<(), CoconutError>;
    fn proposal_id_value(&self) -> Result<u64, CoconutError>;
    async fn is_consistent(&self, epoch_state: EpochState) -> Result<(), CoconutError> {
        match epoch_state {
            EpochState::WaitingInitialisation => {}
            EpochState::PublicKeySubmission { .. } => {}
            EpochState::DealingExchange { .. } => {
                self.node_index_value()?;
            }
            EpochState::VerificationKeySubmission { .. } => {
                self.receiver_index_value()?;
                self.threshold()?;
            }
            EpochState::VerificationKeyValidation { .. } => {
                self.coconut_keypair_is_some().await?;
            }
            EpochState::VerificationKeyFinalization { .. } => {
                self.proposal_id_value()?;
            }
            EpochState::InProgress => {}
        }
        Ok(())
    }
}

#[async_trait]
impl ConsistentState for State {
    fn node_index_value(&self) -> Result<NodeIndex, CoconutError> {
        self.node_index.ok_or(CoconutError::UnrecoverableState {
            reason: String::from("Node index should have been set"),
        })
    }

    fn receiver_index_value(&self) -> Result<usize, CoconutError> {
        self.receiver_index.ok_or(CoconutError::UnrecoverableState {
            reason: String::from("Receiver index should have been set"),
        })
    }

    fn threshold(&self) -> Result<Threshold, CoconutError> {
        let threshold = self.threshold.ok_or(CoconutError::UnrecoverableState {
            reason: String::from("Threshold should have been set"),
        })?;
        if self.current_dealers_by_idx().len() < threshold as usize {
            Err(CoconutError::UnrecoverableState {
                reason: String::from(
                    "Not enough good dealers in the signer set to achieve threshold",
                ),
            })
        } else {
            Ok(threshold)
        }
    }

    async fn coconut_keypair_is_some(&self) -> Result<(), CoconutError> {
        if self.coconut_keypair_is_some().await {
            Ok(())
        } else {
            Err(CoconutError::UnrecoverableState {
                reason: String::from("Coconut keypair should have been set"),
            })
        }
    }

    fn proposal_id_value(&self) -> Result<u64, CoconutError> {
        self.proposal_id.ok_or(CoconutError::UnrecoverableState {
            reason: String::from("Proposal id should have been set"),
        })
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PersistentState {
    timestamp: OffsetDateTime,

    //
    node_index: Option<NodeIndex>,
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    #[serde(with = "generated_dealings_old")]
    generated_dealings: HashMap<EpochId, HashMap<DealingIndex, Dealing>>,
    receiver_index: Option<usize>,
    threshold: Option<Threshold>,
    #[serde(with = "vks_serde")]
    recovered_vks: Vec<RecoveredVerificationKeys>,
    proposal_id: Option<u64>,
    voted_vks: bool,
    executed_proposal: bool,
    was_in_progress: bool,
}

impl Default for PersistentState {
    fn default() -> Self {
        PersistentState {
            timestamp: OffsetDateTime::now_utc(),
            node_index: None,
            dealers: Default::default(),
            generated_dealings: Default::default(),
            receiver_index: None,
            threshold: None,
            recovered_vks: vec![],
            proposal_id: None,
            voted_vks: false,
            executed_proposal: false,
            was_in_progress: false,
        }
    }
}

impl From<&State> for PersistentState {
    fn from(s: &State) -> Self {
        PersistentState {
            timestamp: OffsetDateTime::now_utc(),
            node_index: s.node_index,
            dealers: s.dealers.clone(),
            generated_dealings: s.generated_dealings.clone(),
            receiver_index: s.receiver_index,
            threshold: s.threshold,
            recovered_vks: s.recovered_vks.clone(),
            proposal_id: s.proposal_id,
            voted_vks: s.voted_vks,
            executed_proposal: s.executed_proposal,
            was_in_progress: s.was_in_progress,
        }
    }
}

impl PersistentState {
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CoconutError> {
        debug!("persisting the dkg state");
        std::fs::write(path, serde_json::to_string(self)?)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoconutError> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

#[derive(Default)]
pub(crate) struct DkgState {
    pub(crate) registration: RegistrationState,
    
    pub(crate) dealing_exchange: DealingExchangeState,

    pub(crate) key_generation: KeyDerivationState,
    
    pub(crate) key_validation: ValidationState,
    
    pub(crate) key_finalization: FinalizationState,
}

impl DkgState {
    pub(crate) fn set_dealers(&mut self, raw_dealers: Vec<DealerDetails>) {
        assert!(self.dealing_exchange.dealers.is_empty());
        for raw_dealer in raw_dealers {
            let dkg_participant = DkgParticipant::from(raw_dealer);
            if let ParticipantState::Invalid(complaint) = &dkg_participant.state {
                warn!(
                    "{} dealer is malformed: {complaint}",
                    dkg_participant.address
                )
            }
            self.dealing_exchange.dealers
                .insert(dkg_participant.assigned_index, dkg_participant);
        }
    }
}

pub(crate) struct State {
    /// Path to the file containing the persistent state
    persistent_state_path: PathBuf,

    dkg_instances: HashMap<EpochId, DkgState>,

    //
    #[deprecated]
    announce_address: Url,
    #[deprecated]
    identity_key: identity::PublicKey,
    #[deprecated]
    dkg_keypair: DkgKeyPair,
    #[deprecated]
    coconut_keypair: CoconutKeyPair,
    #[deprecated]
    node_index: Option<NodeIndex>,
    #[deprecated]
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    #[deprecated]
    generated_dealings: HashMap<EpochId, HashMap<DealingIndex, Dealing>>,
    #[deprecated]
    receiver_index: Option<usize>,
    #[deprecated]
    threshold: Option<Threshold>,
    #[deprecated]
    recovered_vks: Vec<RecoveredVerificationKeys>,
    #[deprecated]
    proposal_id: Option<u64>,
    #[deprecated]
    voted_vks: bool,
    #[deprecated]
    executed_proposal: bool,
    #[deprecated]
    was_in_progress: bool,
}

impl State {
    pub fn new(
        persistent_state_path: PathBuf,
        persistent_state: PersistentState,
        announce_address: Url,
        dkg_keypair: DkgKeyPair,
        identity_key: identity::PublicKey,
        coconut_keypair: CoconutKeyPair,
    ) -> Self {
        State {
            persistent_state_path,
            dkg_instances: Default::default(),
            announce_address,
            identity_key,
            dkg_keypair,
            coconut_keypair,
            node_index: persistent_state.node_index,
            dealers: persistent_state.dealers,
            generated_dealings: persistent_state.generated_dealings,
            receiver_index: persistent_state.receiver_index,
            threshold: persistent_state.threshold,
            recovered_vks: persistent_state.recovered_vks,
            proposal_id: persistent_state.proposal_id,
            voted_vks: persistent_state.voted_vks,
            executed_proposal: persistent_state.executed_proposal,
            was_in_progress: persistent_state.was_in_progress,
        }
    }

    pub fn persist(&self) -> Result<(), CoconutError> {
        PersistentState::from(self).save_to_file(self.persistent_state_path())
    }

    pub async fn reset_persistent(&mut self, reset_coconut_keypair: bool) {
        if reset_coconut_keypair {
            self.coconut_keypair.invalidate();
        }
        self.node_index = Default::default();
        self.dealers = Default::default();
        self.receiver_index = Default::default();
        self.threshold = Default::default();
        self.recovered_vks = Default::default();
        self.proposal_id = Default::default();
        self.voted_vks = Default::default();
        self.executed_proposal = Default::default();
        self.was_in_progress = Default::default();
    }

    pub fn maybe_init_dkg_state(&mut self, epoch_id: EpochId) {
        if !self.dkg_instances.contains_key(&epoch_id) {
            self.dkg_instances.insert(epoch_id, Default::default());
        }
    }

    /// Obtain the list of dealers for the provided epoch that have submitted valid public keys.
    pub fn valid_epoch_receivers(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<(Addr, NodeIndex)>, CoconutError> {
        Ok(self
            .dealing_exchange_state(epoch_id)?
            .dealers
            .values()
            .filter_map(|d| {
                if d.state.is_valid() {
                    Some((d.address.clone(), d.assigned_index))
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn valid_epoch_receivers_keys(
        &self,
        epoch_id: EpochId,
    ) -> Result<BTreeMap<NodeIndex, bte::PublicKey>, CoconutError> {
        Ok(self
            .dealing_exchange_state(epoch_id)?
            .dealers
            .values()
            .filter_map(|d| d.state.public_key().map(|k| (d.assigned_index, k)))
            .collect())
    }

    pub fn dkg_state(&self, epoch_id: EpochId) -> Result<&DkgState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn dkg_state_mut(&mut self, epoch_id: EpochId) -> Result<&mut DkgState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn registration_state(
        &self,
        epoch_id: EpochId,
    ) -> Result<&RegistrationState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.registration)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn registration_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut RegistrationState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.registration)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn dealing_exchange_state(
        &self,
        epoch_id: EpochId,
    ) -> Result<&DealingExchangeState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.dealing_exchange)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn dealing_exchange_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut DealingExchangeState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.dealing_exchange)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_derivation_state(
        &self,
        epoch_id: EpochId,
    ) -> Result<&KeyDerivationState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.key_generation)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_derivation_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut KeyDerivationState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.key_generation)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_validation_state(
        &self,
        epoch_id: EpochId,
    ) -> Result<&ValidationState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.key_validation)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_validation_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut ValidationState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.key_validation)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_finalization_state(
        &self,
        epoch_id: EpochId,
    ) -> Result<&FinalizationState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.key_finalization)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn key_finalization_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut FinalizationState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.key_finalization)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn threshold(&self, epoch_id: EpochId) -> Result<Threshold, CoconutError> {
        self.key_derivation_state(epoch_id)?
            .expected_threshold
            .ok_or(CoconutError::UnavailableThreshold { epoch_id })
    }

    pub fn assigned_index(&self, epoch_id: EpochId) -> Result<NodeIndex, CoconutError> {
        self.registration_state(epoch_id)?
            .assigned_index
            .ok_or(CoconutError::UnavailableAssignedIndex { epoch_id })
    }

    pub fn receiver_index(&self, epoch_id: EpochId) -> Result<usize, CoconutError> {
        self.dealing_exchange_state(epoch_id)?
            .receiver_index
            .ok_or(CoconutError::UnavailableReceiverIndex { epoch_id })
    }
    
    pub fn proposal_id(&self, epoch_id: EpochId) -> Result<u64, CoconutError>  {
        self.key_derivation_state(epoch_id)?.proposal_id.ok_or(CoconutError::UnavailableProposalId {epoch_id})
    }

    pub fn persistent_state_path(&self) -> &Path {
        self.persistent_state_path.as_path()
    }

    pub fn announce_address(&self) -> &Url {
        &self.announce_address
    }

    pub fn identity_key(&self) -> identity::PublicKey {
        self.identity_key
    }

    pub fn dkg_keypair(&self) -> &DkgKeyPair {
        &self.dkg_keypair
    }

    pub async fn coconut_keypair_is_some(&self) -> bool {
        self.coconut_keypair.get().await.is_some()
    }

    pub async fn take_coconut_keypair(&self) -> Option<KeyPairWithEpoch> {
        self.coconut_keypair.take().await
    }

    pub fn invalidate_coconut_keypair(&self) {
        self.coconut_keypair.invalidate()
    }

    pub fn get_dealing(&self, epoch_id: EpochId, dealing_index: DealingIndex) -> Option<&Dealing> {
        self.generated_dealings
            .get(&epoch_id)
            .and_then(|epoch_dealings| epoch_dealings.get(&dealing_index))
    }

    pub fn store_dealing(
        &mut self,
        epoch_id: EpochId,
        dealing_index: DealingIndex,
        dealing: Dealing,
    ) {
        self.generated_dealings
            .entry(epoch_id)
            .or_default()
            .insert(dealing_index, dealing);
    }

    pub async fn coconut_keypair(
        &self,
    ) -> Option<tokio::sync::RwLockReadGuard<'_, Option<KeyPairWithEpoch>>> {
        self.coconut_keypair.get().await
    }

    pub fn node_index(&self) -> Option<NodeIndex> {
        self.node_index
    }

    pub fn current_dealers_by_addr(&self) -> BTreeMap<Addr, NodeIndex> {
        self.dealers
            .iter()
            .filter_map(|(addr, dealer)| {
                dealer
                    .as_ref()
                    .ok()
                    .map(|participant| (addr.clone(), participant.assigned_index))
            })
            .collect()
    }

    // FIXME: BUG: if we remove dealers, we won't be able to verify shares of other parties...
    pub fn current_dealers_by_idx(&self) -> BTreeMap<NodeIndex, PublicKey> {
        todo!()
        // self.dealers
        //     .iter()
        //     .filter_map(|(_, dealer)| {
        //         dealer.as_ref().ok().map(|participant| {
        //             (
        //                 participant.assigned_index,
        //                 *participant.bte_public_key_with_proof.public_key(),
        //             )
        //         })
        //     })
        //     .collect()
    }

    pub fn recovered_vks(&self) -> &Vec<RecoveredVerificationKeys> {
        &self.recovered_vks
    }

    pub fn voted_vks(&self) -> bool {
        self.voted_vks
    }

    pub fn executed_proposal(&self) -> bool {
        self.executed_proposal
    }

    pub fn was_in_progress(&self) -> bool {
        self.was_in_progress
    }

    pub fn set_recovered_vks(&mut self, recovered_vks: Vec<RecoveredVerificationKeys>) {
        self.recovered_vks = recovered_vks;
    }

    pub async fn set_coconut_keypair(&mut self, coconut_keypair: KeyPairWithEpoch) {
        self.coconut_keypair.set(coconut_keypair).await
    }

    pub fn set_node_index(&mut self, node_index: Option<NodeIndex>) {
        self.node_index = node_index;
    }

    // pub fn set_dealers(&mut self, dealers: Vec<DealerDetails>) {
    //     self.dealers = BTreeMap::from_iter(
    //         dealers
    //             .into_iter()
    //             .map(|details| (details.address.clone(), DkgParticipant::try_from(details))),
    //     )
    // }

    pub fn mark_bad_dealer(&mut self, dealer_addr: &Addr, reason: ComplaintReason) {
        if let Some((_, value)) = self
            .dealers
            .iter_mut()
            .find(|(addr, _)| *addr == dealer_addr)
        {
            debug!(
                "Dealer {dealer_addr} misbehaved: {reason:?}. It will be marked locally as bad dealer and ignored",
            );
            *value = Err(reason);
        }
    }

    pub fn set_receiver_index(&mut self, receiver_index: Option<usize>) {
        self.receiver_index = receiver_index;
    }

    pub fn set_threshold(&mut self, threshold: Option<Threshold>) {
        self.threshold = threshold;
    }

    pub fn set_proposal_id(&mut self, proposal_id: u64) {
        self.proposal_id = Some(proposal_id);
    }

    pub fn set_voted_vks(&mut self) {
        self.voted_vks = true;
    }

    pub fn set_executed_proposal(&mut self) {
        self.executed_proposal = true;
    }

    pub fn set_was_in_progress(&mut self) {
        self.was_in_progress = true;
    }

    pub fn all_dealers(&self) -> &BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>> {
        &self.dealers
    }
}
