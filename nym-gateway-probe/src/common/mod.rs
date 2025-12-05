// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Common utilities shared across test modes.
//!
//! This module contains shared functionality used by multiple test modes:
//! - WireGuard tunnel testing via netstack

pub mod wireguard;

pub use wireguard::{
    run_tunnel_tests, run_two_hop_tunnel_tests, TwoHopWgTunnelConfig, WgTunnelConfig,
};
