// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::complaints::ComplaintReason;
use coconut_dkg_common::dealer::DealerDetails;
use cosmwasm_std::Addr;
use dkg::bte::{keys::KeyPair, PublicKey, PublicKeyWithProof};
use dkg::{NodeIndex, Share};
use std::collections::BTreeMap;

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Debug)]
pub(crate) struct DkgParticipant {
    pub(crate) address: Addr,
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
            address: dealer.address,
            bte_public_key_with_proof,
            assigned_index: dealer.assigned_index,
        })
    }
}

pub(crate) struct State {
    keypair: KeyPair,
    node_index: Option<NodeIndex>,
    dealers: BTreeMap<Addr, Result<DkgParticipant, ComplaintReason>>,
    self_share: Option<Share>,
}

impl State {
    pub fn new(keypair: KeyPair) -> Self {
        State {
            keypair,
            node_index: None,
            dealers: BTreeMap::new(),
            self_share: None,
        }
    }

    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    pub fn node_index(&self) -> Option<NodeIndex> {
        self.node_index
    }

    pub fn current_receivers(&self) -> BTreeMap<NodeIndex, PublicKey> {
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

    pub fn self_share(&self) -> Option<&Share> {
        self.self_share.as_ref()
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

    pub fn set_self_share(&mut self, self_share: Option<Share>) {
        self.self_share = self_share;
    }
}
