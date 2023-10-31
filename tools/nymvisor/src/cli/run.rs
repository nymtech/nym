// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tokio::runtime;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(trailing_var_arg = true)]
    // #[clap(raw = true)]
    daemon_args: Vec<String>,
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    // TODO: experiment with the minimal runtime
    let rt = runtime::Builder::new_current_thread().enable_io().build()?;

    // spawn the root task
    rt.block_on(async {
        println!("run");

        let mut child = tokio::process::Command::new("echo")
            .args(args.daemon_args)
            .spawn()?;

        let status = child.wait().await?;

        println!("{:?}", status);
        Ok(())
    })
}
