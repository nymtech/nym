// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use colored::Colorize;

#[allow(dead_code)]
#[derive(clap::Args, Clone)]
pub(crate) struct Init {
    /// Id of the mixnode we want to create config for
    #[clap(long)]
    id: Option<String>,
}

pub(crate) fn execute(_args: &Init) -> anyhow::Result<()> {
    bail!(
        "standalone mixnode initialisation has been removed - please initialise a `nym-node` instead"
            .red()
            .bold()
    )
}
