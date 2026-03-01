// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// obviously most of the features currently reside in the gateway,
// but let's start putting everything in here

pub mod error;

pub use nym_gateway::node::wireguard::{PeerManager, PeerRegistrator};
pub use nym_wireguard::{PeerControlRequest, WireguardGatewayData};
