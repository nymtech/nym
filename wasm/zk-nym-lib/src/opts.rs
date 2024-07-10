// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_network_defaults::NymNetworkDetails;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Tsify, Debug, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CredentialClientOpts {
    #[tsify(optional)]
    pub network_details: Option<NymNetworkDetails>,

    #[tsify(optional)]
    pub use_sandbox: Option<bool>,
}
