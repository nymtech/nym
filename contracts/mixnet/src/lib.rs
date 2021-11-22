// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod contract;
pub mod error;
pub(crate) mod helpers;
pub mod queries;
pub mod state;
pub(crate) mod storage;
pub mod support;
pub mod transactions;

use cosmwasm_std::{Addr, Coin, Uint128};
use mixnet_contract::{Layer, MixNode, MixNodeBond};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct StoredMixnodeBond {
    pub bond_amount: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub block_height: u64,
    pub mix_node: MixNode,
    pub profit_margin_percent: Option<u8>,
}

impl StoredMixnodeBond {
    pub(crate) fn new(
        bond_amount: Coin,
        owner: Addr,
        layer: Layer,
        block_height: u64,
        mix_node: MixNode,
        profit_margin_percent: Option<u8>,
    ) -> Self {
        StoredMixnodeBond {
            bond_amount,
            owner,
            layer,
            block_height,
            mix_node,
            profit_margin_percent,
        }
    }

    pub(crate) fn attach_delegation(self, total_delegation: Uint128) -> MixNodeBond {
        MixNodeBond {
            total_delegation: Coin {
                denom: self.bond_amount.denom.clone(),
                amount: total_delegation,
            },
            bond_amount: self.bond_amount,
            owner: self.owner,
            layer: self.layer,
            block_height: self.block_height,
            mix_node: self.mix_node,
            profit_margin_percent: self.profit_margin_percent,
        }
    }

    pub(crate) fn identity(&self) -> &String {
        &self.mix_node.identity_key
    }

    pub(crate) fn bond_amount(&self) -> Coin {
        self.bond_amount.clone()
    }
}

impl Display for StoredMixnodeBond {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "amount: {}, owner: {}, identity: {}",
            self.bond_amount, self.owner, self.mix_node.identity_key
        )
    }
}
