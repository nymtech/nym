// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::tendermint::{block, merkle::proof::ProofOps};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProvableAbciQueryResponse<Res> {
    pub response: Res,

    pub height: block::Height,

    pub proof: ProofOps,
}

impl<R> ProvableAbciQueryResponse<R> {
    pub fn map<T, F: FnOnce(R) -> T>(self, op: F) -> ProvableAbciQueryResponse<T> {
        ProvableAbciQueryResponse {
            response: op(self.response),
            height: self.height,
            proof: self.proof,
        }
    }
}
