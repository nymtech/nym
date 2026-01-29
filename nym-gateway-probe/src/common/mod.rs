// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Common utilities shared across test modes.
//!
//! This module contains shared functionality used by multiple test modes:
//! - WireGuard tunnel testing via netstack

pub(crate) mod bandwidth_helpers;
pub(crate) mod helpers;
pub(crate) mod icmp;
pub(crate) mod netstack;
pub(crate) mod nodes;
pub(crate) mod probe_tests;
pub(crate) mod socks5_test;
pub(crate) mod types;
pub(crate) mod wireguard;
