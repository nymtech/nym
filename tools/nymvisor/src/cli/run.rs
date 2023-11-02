// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_load_current_config;
use crate::env::Env;
use crate::error::NymvisorError;
use nym_bin_common::logging::setup_tracing_logger;
use tokio::runtime;
use tracing::info;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(trailing_var_arg = true)]
    // #[clap(raw = true)]
    daemon_args: Vec<String>,
}

pub(crate) fn execute(args: Args) -> Result<(), NymvisorError> {
    let env = Env::try_read()?;
    let config = try_load_current_config(&env)?;
    if !config.nymvisor.debug.disable_logs {
        setup_tracing_logger();
    }

    info!("starting nymvisor for {}", config.daemon.name);

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
