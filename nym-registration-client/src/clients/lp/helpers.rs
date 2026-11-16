// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning registration messages into LP frames and back.
//!
//! The channel these travel over knows nothing about them - see
//! [`nym_lp_gateway_client`].

use nym_lp::peer::LpRemotePeer;
use nym_registration_common::NymNodeLPInformation;

pub(crate) fn to_lp_remote_peer(data: NymNodeLPInformation) -> LpRemotePeer {
    LpRemotePeer::new(data.x25519).with_key_digests(data.expected_kem_key_hashes)
}
