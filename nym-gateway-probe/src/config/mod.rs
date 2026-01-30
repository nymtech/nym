// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Args;

mod credentials;
mod netstack;
mod socks5;
mod test_mode;

pub use credentials::{CredentialArgs, CredentialMode};
pub use netstack::NetstackArgs;
pub use socks5::Socks5Args;
pub use test_mode::TestMode;

#[derive(Args, Debug)]
pub struct ProbeConfig {
    /// Only choose gateway with that minimum performance
    #[arg(long)]
    pub min_gateway_mixnet_performance: Option<u8>,

    /// Test mode - explicitly specify which tests to run
    ///
    /// Modes:
    ///   core.       - Traditional mixnet testing (entry/exit pings + WireGuard via authenticator)
    ///   wg-mix      - Wireguard via authenticator
    ///   wg-lp       - Entry LP + Exit LP (nested forwarding) + WireGuard
    ///   lp-only     - LP registration only (no WireGuard)
    ///   socks5-only - Socks5 network requester test
    ///   all         - Mixnet, wireguard over authenticator and LP registration
    ///
    #[arg(long, default_value_t = TestMode::default(), verbatim_doc_comment)]
    pub test_mode: TestMode,

    #[arg(long, global = true)]
    pub ignore_egress_epoch_role: bool,

    /// Arguments to be appended to the wireguard config enabling amnezia-wg configuration
    #[arg(long)]
    pub amnezia_args: Option<String>,

    /// Arguments to manage netstack downloads
    #[command(flatten)]
    pub netstack_args: NetstackArgs,

    /// Arguments to configure socks5 probe
    #[command(flatten)]
    pub socks5_args: Socks5Args,
}
