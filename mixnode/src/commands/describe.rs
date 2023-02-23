// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::node_description::NodeDescription;
use clap::Args;
use colored::Colorize;
use nym_config::NymConfig;
use std::io;
use std::io::Write;

#[derive(Args)]
pub(crate) struct Describe {
    /// The id of the mixnode you want to describe
    #[clap(long)]
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

pub(crate) fn execute(args: Describe) {
    // ensure that the mixnode has in fact been initialized
    match Config::load_from_file(&args.id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {err})", &args.id);
            return;
        }
    };

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
    NodeDescription::save_to_file(
        &node_description,
        Config::default_config_directory(&args.id),
    )
    .unwrap()
}
