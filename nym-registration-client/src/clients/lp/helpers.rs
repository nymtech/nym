// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Odds and ends for registering over LP.
//!
//! Turning registration messages into LP frames is the channel's business and lives in
//! [`nym_lp_gateway_client`]; what is left here is what only the registration client knows.

use nym_lp::peer::LpRemotePeer;
use nym_registration_common::NymNodeLPInformation;

pub(crate) fn to_lp_remote_peer(data: NymNodeLPInformation) -> LpRemotePeer {
    LpRemotePeer::new(data.x25519).with_key_digests(data.expected_kem_key_hashes)
}
