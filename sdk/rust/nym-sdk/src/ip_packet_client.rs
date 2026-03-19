// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Minimal fork of `nym-ip-packet-client` for use by [`IpMixStream`](crate::ipr_wrapper::IpMixStream).
//!
//! Contains only what IpMixStream needs: IPR discovery and response parsing.
//! The full crate lives in `nym-vpn-client`.

pub mod discovery;
pub mod listener;

pub use listener::{handle_ipr_response, MixnetMessageOutcome};

// Re-export the currently used version
pub use nym_ip_packet_requests::v8 as current;
