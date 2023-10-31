// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(trailing_var_arg = true)]
    // #[clap(raw = true)]
    daemon_args: Vec<String>,
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    println!("run");

    let mut child = std::process::Command::new("echo")
        .args(args.daemon_args)
        .spawn()?;

    let status = child.wait()?;

    println!("{:?}", status);
    Ok(())
}
