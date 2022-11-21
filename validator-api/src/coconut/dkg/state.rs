// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::error::CoconutError;
use crate::coconut::keypair::KeyPair as CoconutKeyPair;
use coconut_dkg_common::dealer::DealerDetails;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::Addr;
use dkg::bte::{keys::KeyPair as DkgKeyPair, PublicKey, PublicKeyWithProof};
use dkg::{NodeIndex, Threshold};
use std::collections::BTreeMap;

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

        Ok(DkgParticipant {
            _address: dealer.address,
            bte_public_key_with_proof,
            assigned_index: dealer.assigned_index,
        })
    }
}

pub(crate) trait ConsistentState {
    fn node_index_value(&self) -> Result<NodeIndex, CoconutError>;
    fn receiver_index_value(&self) -> Result<usize, CoconutError>;
    fn threshold(&self) -> Result<Threshold, CoconutError>;
    fn is_consistent(&self, epoch_state: EpochState) -> Result<(), CoconutError> {
        match epoch_state {
            EpochState::PublicKeySubmission => {}
            EpochState::DealingExchange => {
                self.node_index_value()?;
            }
            EpochState::VerificationKeySubmission => {
                self.node_index_value()?;
                self.receiver_index_value()?;
                self.threshold()?;
            }
            EpochState::InProgress => {
                self.node_index_value()?;
                self.receiver_index_value()?;
                self.threshold()?;
            }
        }
        Ok(())
    }
}

pub(crate) struct State {
    dkg_keypair: DkgKeyPair,
    coconut_keypair: CoconutKeyPair,
    node_index: Option<NodeIndex>,
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    receiver_index: Option<usize>,
    threshold: Option<Threshold>,
}

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
}

impl State {
    pub fn new(dkg_keypair: DkgKeyPair, coconut_keypair: CoconutKeyPair) -> Self {
        State {
            dkg_keypair,
            coconut_keypair,
            node_index: None,
            dealers: BTreeMap::new(),
            receiver_index: None,
            threshold: None,
        }
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

    pub async fn set_coconut_keypair(&mut self, coconut_keypair: coconut_interface::KeyPair) {
        self.coconut_keypair.set(coconut_keypair).await
    }

    pub fn set_node_index(&mut self, node_index: NodeIndex) {
        self.node_index = Some(node_index);
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
}
