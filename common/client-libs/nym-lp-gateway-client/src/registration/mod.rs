// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a client says over an LP channel to become known to a gateway.
//!
//! The transport in this crate moves packets and holds no session; a registration client borrows a
//! channel, owns a session, and does one request/response over it. [`exchange_registration`] is
//! that round trip, shared by every mode.
//!
//! # Why it lives here rather than in `nym-registration-client`
//!
//! Because `nym-client-core` needs to register at startup and cannot reach that crate:
//! `nym-registration-client` -> `nym-sdk` -> `nym-client-core` is a cycle, and feature-gating does
//! not help since Cargo resolves optional dependencies too.
//!
//! This is temporary. Once the legacy websocket path leaves `nym-registration-client`, it sheds
//! `nym-sdk`, drops below `nym-client-core`, and this module moves there to sit beside dVPN
//! registration - which stays where it is in the meantime, with the credential and WireGuard
//! machinery it needs.

pub use frames::{LpFrameDeliverExt, LpFrameSendExt, exchange_registration};
pub use mixnet::LpMixnetRegistrationClient;

mod frames;
mod mixnet;
