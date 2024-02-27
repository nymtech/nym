// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::DEFAULT_MIXNODE_ID;
use crate::commands::try_load_current_config;
use crate::env::vars::*;
use crate::node::node_description::NodeDescription;
use clap::Args;
use colored::Colorize;
use std::io;
use std::io::Write;

#[derive(Args)]
pub(crate) struct Describe {
    /// The id of the mixnode you want to describe
    #[clap(long, default_value = DEFAULT_MIXNODE_ID, env = MIXNODE_ID_ARG)]
    id: String,

    /// Human readable name of this node
    #[clap(long)]
    name: Option<String>,

    /// Description of this node
    #[clap(long)]
    description: Option<String>,

    /// Link associated with this node, for example `https://mixnode.yourdomain.com`
    #[clap(long)]
    link: Option<String>,

    /// Physical location of this node, for example `City: London, Country: UK`
    #[clap(long)]
    location: Option<String>,
}

fn read_user_input() -> String {
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

pub(crate) fn execute(args: Describe) -> anyhow::Result<()> {
    // ensure that the mixnode has in fact been initialized
    let config = try_load_current_config(&args.id)?;

    let example_url = "https://mixnode.yourdomain.com".bright_cyan();
    let example_location = "City: London, Country: UK";

    // get input from the user if not provided via the arguments
    let name = args.name.unwrap_or_else(|| {
        print!("name: ");
        read_user_input()
    });

    let description = args.description.unwrap_or_else(|| {
        print!("description: ");
        read_user_input()
    });

    let link = args.link.unwrap_or_else(|| {
        print!("link, e.g. {example_url}: ");
        read_user_input()
    });

    let location = args.location.unwrap_or_else(|| {
        print!("location, e.g. {example_location}: ");
        read_user_input()
    });

    let node_description = NodeDescription {
        name,
        description,
        link,
        location,
    };

    // save the struct
    NodeDescription::save_to_file(&node_description, config.storage_paths.node_description)?;
    Ok(())
}
