// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use tokio::runtime;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(trailing_var_arg = true)]
    // #[clap(raw = true)]
    daemon_args: Vec<String>,
}

pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    // TODO: experiment with the minimal runtime
    // look at futures::executor::LocalPool
    let rt = runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();

    // spawn the root task
    rt.block_on(async {
        println!("run");

        let mut child = tokio::process::Command::new("echo")
            .args(args.daemon_args)
            .spawn()
            .unwrap();

        let status = child.wait().await.unwrap();

        println!("{:?}", status);
        Ok(())
    })
}
