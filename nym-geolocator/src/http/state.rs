// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nyx::nodes::BondedNymNodes;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) bonded_nodes: BondedNymNodes,
}
