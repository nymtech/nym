// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_load_current_config;
use crate::daemon::Daemon;
use crate::env::Env;
use crate::error::NymvisorError;
use nym_bin_common::logging::setup_tracing_logger;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime;
use tokio::sync::Notify;
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
    // well, if the creation of the runtime failed, there isn't much we could do
    #[allow(clippy::expect_used)]
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create the runtime");

    // spawn the root task
    rt.block_on(async {
        println!("run");

        let daemon = Daemon::from_config(&config).with_kill_timeout(Duration::from_millis(200));
        let interrupt_notify = Arc::new(Notify::new());
        let running = daemon.execute_async(args.daemon_args, Arc::clone(&interrupt_notify))?;

        let handle = tokio::spawn(async move {
            let res = running.await;
            println!("the process has finished! with {res:?}");
        });

        tokio::time::sleep(Duration::from_secs(2)).await;
        info!(">>>>>>>>>> NYMVISOR: sending interrupt to the daemon");

        interrupt_notify.notify_one();

        handle.await;

        // println!("{:?}", status);
        Ok(())
    })
}
