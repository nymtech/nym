// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use tsify::Tsify;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Tsify, Serialize, Deserialize, Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct SerialisedNymTicketBook {
    pub serialisation_revision: u8,
    pub bs58_encoded_data: String,
}
