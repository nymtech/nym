// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! High-level IPR (IP Packet Router) stream wrapper.
//!
//! [`IpMixStream`] tunnels IP packets through the Nym mixnet to an exit
//! gateway running an IP Packet Router. Both requests and responses are
//! wrapped in LP Stream frames for type-safe detection at the IPR and
//! dispatch by the client's stream router.

mod ip_mix_stream;
pub mod network_env;

pub use ip_mix_stream::IpMixStream;
pub use network_env::NetworkEnvironment;
