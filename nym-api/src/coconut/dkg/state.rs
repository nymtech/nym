// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::error::CoconutError;
use crate::coconut::keypair::KeyPair as CoconutKeyPair;
use cosmwasm_std::Addr;
use log::debug;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::types::EpochState;
use nym_dkg::bte::{keys::KeyPair as DkgKeyPair, PublicKey, PublicKeyWithProof};
use nym_dkg::{NodeIndex, RecoveredVerificationKeys, Threshold};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use url::Url;

fn bte_pk_serialize<S: Serializer>(
    val: &PublicKeyWithProof,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    val.to_bytes().serialize(serializer)
}

fn bte_pk_deserialize<'de, D>(deserializer: D) -> Result<PublicKeyWithProof, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<u8> = Deserialize::deserialize(deserializer)?;
    PublicKeyWithProof::try_from_bytes(&vec).map_err(|err| Error::custom(format_args!("{:?}", err)))
}

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) struct DkgParticipant {
    pub(crate) _address: Addr,
    #[serde(serialize_with = "bte_pk_serialize")]
    #[serde(deserialize_with = "bte_pk_deserialize")]
    pub(crate) bte_public_key_with_proof: PublicKeyWithProof,
    pub(crate) assigned_index: NodeIndex,
}

impl TryFrom<DealerDetails> for DkgParticipant {
    type Error = ComplaintReason;

    fn try_from(dealer: DealerDetails) -> Result<Self, Self::Error> {
        let bte_public_key_with_proof = bs58::decode(dealer.bte_public_key_with_proof)
            .into_vec()
            .map(|bytes| PublicKeyWithProof::try_from_bytes(&bytes))
            .map_err(|_| ComplaintReason::MalformedBTEPublicKey)?
            .map_err(|_| ComplaintReason::MalformedBTEPublicKey)?;

        if !bte_public_key_with_proof.verify() {
            return Err(ComplaintReason::InvalidBTEPublicKey);
        }

        Ok(DkgParticipant {
            _address: dealer.address,
            bte_public_key_with_proof,
            assigned_index: dealer.assigned_index,
        })
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

fn vks_serialize<S: Serializer>(
    val: &[RecoveredVerificationKeys],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let vec: Vec<Vec<u8>> = val.iter().map(|vk| vk.to_bytes()).collect();
    vec.serialize(serializer)
}

fn vks_deserialize<'de, D>(deserializer: D) -> Result<Vec<RecoveredVerificationKeys>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<Vec<u8>> = Deserialize::deserialize(deserializer)?;
    vec.into_iter()
        .map(|b| {
            RecoveredVerificationKeys::try_from_bytes(&b)
                .map_err(|err| D::Error::custom(format_args!("{:?}", err)))
        })
        .collect()
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct PersistentState {
    node_index: Option<NodeIndex>,
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    receiver_index: Option<usize>,
    threshold: Option<Threshold>,
    #[serde(serialize_with = "vks_serialize")]
    #[serde(deserialize_with = "vks_deserialize")]
    recovered_vks: Vec<RecoveredVerificationKeys>,
    proposal_id: Option<u64>,
    voted_vks: bool,
    executed_proposal: bool,
    was_in_progress: bool,
}

impl From<&State> for PersistentState {
    fn from(s: &State) -> Self {
        PersistentState {
            node_index: s.node_index,
            dealers: s.dealers.clone(),
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
        std::fs::write(path, serde_json::to_string(self)?)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoconutError> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

pub(crate) struct State {
    persistent_state_path: PathBuf,
    announce_address: Url,
    dkg_keypair: DkgKeyPair,
    coconut_keypair: CoconutKeyPair,
    node_index: Option<NodeIndex>,
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    receiver_index: Option<usize>,
    threshold: Option<Threshold>,
    recovered_vks: Vec<RecoveredVerificationKeys>,
    proposal_id: Option<u64>,
    voted_vks: bool,
    executed_proposal: bool,
    was_in_progress: bool,
}

impl State {
    pub fn new(
        persistent_state_path: PathBuf,
        persistent_state: PersistentState,
        announce_address: Url,
        dkg_keypair: DkgKeyPair,
        coconut_keypair: CoconutKeyPair,
    ) -> Self {
        State {
            persistent_state_path,
            announce_address,
            dkg_keypair,
            coconut_keypair,
            node_index: persistent_state.node_index,
            dealers: persistent_state.dealers,
            receiver_index: persistent_state.receiver_index,
            threshold: persistent_state.threshold,
            recovered_vks: persistent_state.recovered_vks,
            proposal_id: persistent_state.proposal_id,
            voted_vks: persistent_state.voted_vks,
            executed_proposal: persistent_state.executed_proposal,
            was_in_progress: persistent_state.was_in_progress,
        }
    }

    pub async fn reset_persistent(&mut self, reset_coconut_keypair: bool) {
        if reset_coconut_keypair {
            self.coconut_keypair.set(None).await;
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

    pub fn persistent_state_path(&self) -> PathBuf {
        self.persistent_state_path.clone()
    }

    pub fn announce_address(&self) -> &Url {
        &self.announce_address
    }

    pub fn dkg_keypair(&self) -> &DkgKeyPair {
        &self.dkg_keypair
    }

    pub async fn coconut_keypair_is_some(&self) -> bool {
        self.coconut_keypair.get().await.is_some()
    }

    pub async fn take_coconut_keypair(&self) -> Option<nym_coconut::KeyPair> {
        self.coconut_keypair.take().await
    }

    #[cfg(test)]
    pub async fn coconut_keypair(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, Option<nym_coconut::KeyPair>> {
        self.coconut_keypair.get().await
    }

    pub fn node_index(&self) -> Option<NodeIndex> {
        self.node_index
    }

    pub fn receiver_index(&self) -> Option<usize> {
        self.receiver_index
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

    pub fn current_dealers_by_idx(&self) -> BTreeMap<NodeIndex, PublicKey> {
        self.dealers
            .iter()
            .filter_map(|(_, dealer)| {
                dealer.as_ref().ok().map(|participant| {
                    (
                        participant.assigned_index,
                        *participant.bte_public_key_with_proof.public_key(),
                    )
                })
            })
            .collect()
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

    pub async fn set_coconut_keypair(
        &mut self,
        coconut_keypair: Option<nym_coconut_interface::KeyPair>,
    ) {
        self.coconut_keypair.set(coconut_keypair).await
    }

    pub fn set_node_index(&mut self, node_index: Option<NodeIndex>) {
        self.node_index = node_index;
    }

    pub fn set_dealers(&mut self, dealers: Vec<DealerDetails>) {
        self.dealers = BTreeMap::from_iter(
            dealers
                .into_iter()
                .map(|details| (details.address.clone(), DkgParticipant::try_from(details))),
        )
    }

    pub fn mark_bad_dealer(&mut self, dealer_addr: &Addr, reason: ComplaintReason) {
        if let Some((_, value)) = self
            .dealers
            .iter_mut()
            .find(|(addr, _)| *addr == dealer_addr)
        {
            debug!(
                "Dealer {} misbehaved: {:?}. It will be marked locally as bad dealer and ignored",
                dealer_addr, reason
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

    #[cfg(test)]
    pub fn all_dealers(&self) -> &BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>> {
        &self.dealers
    }
}
