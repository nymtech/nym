// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod new_peer_registration;
pub mod peer_manager;

pub use error::GatewayWireguardError;
pub use new_peer_registration::PeerRegistrator;
pub use peer_manager::PeerManager;
