// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::complaints::ComplaintReason;
use coconut_dkg_common::dealer::DealerDetails;
use cosmwasm_std::Addr;
use dkg::bte::{keys::KeyPair, PublicKeyWithProof};
use dkg::{NodeIndex, Share};
use std::collections::HashMap;

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

pub(crate) struct State {
    keypair: KeyPair,
    node_index: Option<NodeIndex>,
    bad_dealers: HashMap<NodeIndex, ComplaintReason>,
    current_dealers: HashMap<NodeIndex, DkgParticipant>,
    self_share: Option<Share>,
}

impl State {
    pub fn new(keypair: KeyPair) -> Self {
        State {
            keypair,
            node_index: None,
            bad_dealers: HashMap::new(),
            current_dealers: HashMap::new(),
            self_share: None,
        }
    }

    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    pub fn node_index(&self) -> Option<NodeIndex> {
        self.node_index
    }

    pub fn current_dealers(&self) -> &HashMap<NodeIndex, DkgParticipant> {
        &self.current_dealers
    }

    pub fn self_share(&self) -> Option<&Share> {
        self.self_share.as_ref()
    }

    pub fn set_node_index(&mut self, node_index: NodeIndex) {
        self.node_index = Some(node_index);
    }

    pub fn add_bad_dealer(&mut self, node_index: NodeIndex, reason: ComplaintReason) {
        self.bad_dealers.insert(node_index, reason);
    }

    pub fn add_good_dealer(&mut self, dealer: DkgParticipant) {
        self.current_dealers.insert(dealer.assigned_index, dealer);
    }

    pub fn set_self_share(&mut self, self_share: Option<Share>) {
        self.self_share = self_share;
    }
}
