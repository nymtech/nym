// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::NodeIndex;
use crypto::asymmetric::identity;
use dkg::bte;
use serde::{Deserialize, Serialize};

// TODO: some TryFrom impl to convert from encoded contract data
struct Dealer {
    // node_index: Option<NodeIndex>,
    // bte_public_key: bte::PublicKey,
    identity: identity::PublicKey,
}

enum DealerState {
    DealingReceived(),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DkgState {
    //
}

impl DkgState {
    // some save/load action here
}
