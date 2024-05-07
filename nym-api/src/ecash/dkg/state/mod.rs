// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::state::dealing_exchange::DealingExchangeState;
use crate::ecash::dkg::state::in_progress::InProgressState;
use crate::ecash::dkg::state::key_derivation::KeyDerivationState;
use crate::ecash::dkg::state::key_finalization::FinalizationState;
use crate::ecash::dkg::state::key_validation::ValidationState;
use crate::ecash::dkg::state::registration::{DkgParticipant, ParticipantState, RegistrationState};
use crate::ecash::error::CoconutError;
use crate::ecash::keys::{KeyPair as CoconutKeyPair, KeyPairWithEpoch};
use cosmwasm_std::Addr;
use log::debug;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::types::EpochId;
use nym_crypto::asymmetric::identity;
use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
use nym_dkg::{bte, NodeIndex, Threshold};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use url::Url;

pub(crate) mod dealing_exchange;
pub(crate) mod in_progress;
pub(crate) mod key_derivation;
pub(crate) mod key_finalization;
pub(crate) mod key_validation;
pub(crate) mod registration;
pub(crate) mod serde_helpers;

#[derive(Deserialize, Serialize)]
pub(crate) struct PersistentState {
    timestamp: OffsetDateTime,

    dkg_instances: HashMap<EpochId, DkgState>,
}

impl Default for PersistentState {
    fn default() -> Self {
        PersistentState {
            timestamp: OffsetDateTime::now_utc(),

            dkg_instances: Default::default(),
        }
    }
}

impl From<&State> for PersistentState {
    fn from(s: &State) -> Self {
        PersistentState {
            timestamp: OffsetDateTime::now_utc(),

            dkg_instances: s.dkg_instances.clone(),
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct DkgState {
    pub(crate) registration: RegistrationState,

    pub(crate) dealing_exchange: DealingExchangeState,

    pub(crate) key_generation: KeyDerivationState,

    pub(crate) key_validation: ValidationState,

    pub(crate) key_finalization: FinalizationState,

    pub(crate) in_progress: InProgressState,
}

impl DkgState {
    pub(crate) fn set_dealers(&mut self, raw_dealers: Vec<DealerDetails>) {
        assert!(self.dealing_exchange.dealers.is_empty());
        for raw_dealer in raw_dealers {
            let dkg_participant = DkgParticipant::from(raw_dealer);
            let address = &dkg_participant.address;
            if let ParticipantState::Invalid(rejection) = &dkg_participant.state {
                warn!("{address} dealer is malformed: {rejection}",)
            }
            self.dealing_exchange
                .dealers
                .insert(dkg_participant.assigned_index, dkg_participant);
        }
    }
}

pub(crate) struct State {
    /// Path to the file containing the persistent state
    persistent_state_path: PathBuf,

    dkg_instances: HashMap<EpochId, DkgState>,

    announce_address: Url,

    identity_key: identity::PublicKey,

    dkg_keypair: DkgKeyPair,

    coconut_keypair: CoconutKeyPair,
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
            dkg_instances: persistent_state.dkg_instances,
            announce_address,
            identity_key,
            dkg_keypair,
            coconut_keypair,
        }
    }

    pub fn persist(&self) -> Result<(), CoconutError> {
        PersistentState::from(self).save_to_file(self.persistent_state_path())
    }

    pub fn clear_previous_epoch(&mut self, current_epoch: EpochId) {
        if let Some(previous) = current_epoch.checked_sub(1) {
            self.dkg_instances.remove(&previous);
        }
    }

    pub fn maybe_init_dkg_state(&mut self, epoch_id: EpochId) {
        // given we're not using that entry here, I think the explicit check and insert is more readable
        #[allow(clippy::map_entry)]
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

    /// Filters out DKG participants based on whether they submitted valid public key
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

    pub fn in_progress_state(&self, epoch_id: EpochId) -> Result<&InProgressState, CoconutError> {
        self.dkg_instances
            .get(&epoch_id)
            .map(|state| &state.in_progress)
            .ok_or(CoconutError::MissingDkgState { epoch_id })
    }

    pub fn in_progress_state_mut(
        &mut self,
        epoch_id: EpochId,
    ) -> Result<&mut InProgressState, CoconutError> {
        self.dkg_instances
            .get_mut(&epoch_id)
            .map(|state| &mut state.in_progress)
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

    pub fn proposal_id(&self, epoch_id: EpochId) -> Result<u64, CoconutError> {
        self.key_derivation_state(epoch_id)?
            .proposal_id
            .ok_or(CoconutError::UnavailableProposalId { epoch_id })
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
        self.coconut_keypair.read_keys().await.is_some()
    }

    pub async fn take_coconut_keypair(&self) -> Option<KeyPairWithEpoch> {
        self.coconut_keypair.take().await
    }

    pub fn invalidate_coconut_keypair(&self) {
        self.coconut_keypair.invalidate()
    }

    pub fn validate_coconut_keypair(&self) {
        self.coconut_keypair.validate()
    }

    pub async fn unchecked_coconut_keypair(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, Option<KeyPairWithEpoch>> {
        self.coconut_keypair.read_keys().await
    }

    pub async fn set_coconut_keypair(&mut self, coconut_keypair: KeyPairWithEpoch) {
        self.coconut_keypair.set(coconut_keypair).await
    }
}
