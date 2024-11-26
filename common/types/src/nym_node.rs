// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use crate::mixnode::NodeRewarding;
use nym_mixnet_contract_common::{NodeId, NymNode, PendingNodeChanges};
use nym_mixnet_contract_common::{
    NymNodeBond as MixnetContractNymNodeBond, NymNodeDetails as MixnetContractNymNodeDetails,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Full details associated with given node.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NymNodeDetails.ts"
    )
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct NymNodeDetails {
    /// Basic bond information of this node, such as owner address, original pledge, etc.
    pub bond_information: NymNodeBond,

    /// Details used for computation of rewarding related data.
    pub rewarding_details: NodeRewarding,

    /// Adjustments to the node that are scheduled to happen during future epoch/interval transitions.
    pub pending_changes: PendingNodeChanges,
}

impl NymNodeDetails {
    pub fn from_mixnet_contract_nym_node_details(
        details: MixnetContractNymNodeDetails,
        reg: &RegisteredCoins,
    ) -> Result<NymNodeDetails, TypesError> {
        Ok(NymNodeDetails {
            bond_information: NymNodeBond::from_mixnet_contract_mixnode_bond(
                details.bond_information,
                reg,
            )?,
            rewarding_details: NodeRewarding::from_mixnet_contract_node_rewarding(
                details.rewarding_details,
                reg,
            )?,
            pending_changes: details.pending_changes,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/NymNodeBond.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct NymNodeBond {
    /// Unique id assigned to the bonded node.
    pub node_id: NodeId,

    /// Address of the owner of this nym-node.
    pub owner: String,

    /// Original amount pledged by the operator of this node.
    pub original_pledge: DecCoin,

    /// Block height at which this nym-node has been bonded.
    pub bonding_height: u64,

    /// Flag to indicate whether this node is in the process of unbonding,
    /// that will conclude upon the epoch finishing.
    pub is_unbonding: bool,

    #[serde(flatten)]
    /// Information provided by the operator for the purposes of bonding.
    pub node: NymNode,
}

impl NymNodeBond {
    pub fn from_mixnet_contract_mixnode_bond(
        bond: MixnetContractNymNodeBond,
        reg: &RegisteredCoins,
    ) -> Result<NymNodeBond, TypesError> {
        Ok(NymNodeBond {
            node_id: bond.node_id,
            owner: bond.owner.into_string(),
            original_pledge: reg
                .attempt_convert_to_display_dec_coin(bond.original_pledge.into())?,
            node: bond.node,
            bonding_height: bond.bonding_height,
            is_unbonding: bond.is_unbonding,
        })
    }
}
