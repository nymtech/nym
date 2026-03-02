// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_lp::peer::DHPublicKey;
use nym_lp::{KEM, KEMKeyDigests};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::sync::Arc;

/// Wrapper around all known LP nodes
pub(crate) struct LpNodes {
    // map between all available ip addresses of other nodes and their details
    nodes: ArcSwap<HashMap<IpAddr, LpNodeDetails>>,
}

struct LpNodeDetails {
    inner: Arc<LpNodeDetailsInner>,
}

struct LpNodeDetailsInner {
    kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
    x25519: DHPublicKey,
}
