// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use clap::Args;
use colored::Colorize;

#[allow(dead_code)]
#[derive(Args, Clone, Debug)]
pub struct Init {
    /// Id of the gateway we want to create config for
    #[clap(long)]
    id: Option<String>,
}

pub async fn execute(_args: Init) -> anyhow::Result<()> {
    bail!(
        "standalone mixnode initialisation has been removed - please initialise a `nym-node` instead"
            .red()
            .bold()
    )
}
