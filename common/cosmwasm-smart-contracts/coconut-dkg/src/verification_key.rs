// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::NodeIndex;
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

pub type VerificationKeyShare = String;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContractVKShare {
    pub share: VerificationKeyShare,
    pub node_index: NodeIndex,
    pub owner: Addr,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PagedVKSharesResponse {
    pub shares: Vec<ContractVKShare>,
    pub per_page: usize,
    pub start_next_after: Option<Addr>,
}
