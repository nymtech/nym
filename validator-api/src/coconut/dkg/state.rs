// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::error::CoconutError;
use crate::coconut::keypair::KeyPair as CoconutKeyPair;
use coconut_dkg_common::dealer::DealerDetails;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Addr;
use dkg::bte::{keys::KeyPair as DkgKeyPair, PublicKey, PublicKeyWithProof};
use dkg::{NodeIndex, RecoveredVerificationKeys, Threshold};
use std::collections::BTreeMap;
use url::Url;

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Debug)]
pub(crate) struct DkgParticipant {
    pub(crate) _address: Addr,
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
            EpochState::PublicKeySubmission => {}
            EpochState::DealingExchange => {
                self.node_index_value()?;
            }
            EpochState::VerificationKeySubmission => {
                self.receiver_index_value()?;
                self.threshold()?;
            }
            EpochState::VerificationKeyValidation => {
                self.coconut_keypair_is_some().await?;
            }
            EpochState::VerificationKeyFinalization => {
                self.proposal_id_value()?;
            }
            EpochState::InProgress => {}
        }
        Ok(())
    }
}

pub(crate) struct State {
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
            reason: String::from("Proposal id should have benn set"),
        })
    }
}

impl State {
    pub fn new(
        announce_address: Url,
        dkg_keypair: DkgKeyPair,
        coconut_keypair: CoconutKeyPair,
    ) -> Self {
        State {
            announce_address,
            dkg_keypair,
            coconut_keypair,
            node_index: None,
            dealers: BTreeMap::new(),
            receiver_index: None,
            threshold: None,
            recovered_vks: vec![],
            proposal_id: None,
            voted_vks: false,
            executed_proposal: false,
        }
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

    pub fn set_recovered_vks(&mut self, recovered_vks: Vec<RecoveredVerificationKeys>) {
        self.recovered_vks = recovered_vks;
    }

    pub async fn set_coconut_keypair(&mut self, coconut_keypair: coconut_interface::KeyPair) {
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
            *value = Err(reason);
        }
    }

    pub fn set_receiver_index(&mut self, receiver_index: Option<usize>) {
        self.receiver_index = receiver_index;
    }

    pub fn set_threshold(&mut self, threshold: Threshold) {
        self.threshold = Some(threshold);
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

    #[cfg(test)]
    pub fn all_dealers(&self) -> &BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>> {
        &self.dealers
    }
}
