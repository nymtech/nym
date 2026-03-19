// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::cli::common::CommonArgs;
use nym_crypto::asymmetric::x25519;
use std::net::SocketAddr;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(flatten)]
    common_args: CommonArgs,

    /// The socket address of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_ADDRESS_ARG)]
    tested_node_address: SocketAddr,

    /// Noise key of the node to test
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NODE_NOISE_KEY_ARG)]
    tested_node_noise_key: x25519::PublicKey,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    todo!()
}
