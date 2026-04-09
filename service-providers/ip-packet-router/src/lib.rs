// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::panic)]
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

pub mod config;
pub mod error;
pub mod request_filter;

pub(crate) mod messages;
pub(crate) mod non_linux_dummy;

mod clients;
mod constants;
mod ip_packet_router;
mod mixnet_client;
mod mixnet_listener;
mod tun_listener;
mod util;

pub use crate::config::Config;
pub use ip_packet_router::{IpPacketRouter, OnStartData};
