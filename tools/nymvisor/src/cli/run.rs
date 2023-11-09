// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_load_current_config;
use crate::env::Env;
use crate::error::NymvisorError;
use crate::tasks::launcher::DaemonLauncher;
use crate::tasks::upgrade_plan_watcher::start_upgrade_plan_watcher;
use crate::tasks::upstream_poller::UpstreamPoller;
use nym_bin_common::logging::setup_tracing_logger;
use tokio::runtime;
use tracing::{error, info};

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

    // we have three tasks only:
    // - one for managing the daemon launcher
    // - the other one for watching the upgrade plan file
    // - the last one for polling upstream source for upgrade info
    // so once the daemon has finished, for whatever reason, abort the file watcher and upstream poller to terminate the nymvisor

    // spawn the root task
    rt.block_on(async {
        let (upgrade_receiver, watcher_handle) = start_upgrade_plan_watcher(&config)?;
        let upstream_poller_handle = UpstreamPoller::new(&config).start();
        let mut launcher = DaemonLauncher::new(config, upgrade_receiver);

        if let Err(err) = launcher.run_loop(args.daemon_args).await {
            error!("the daemon could not continue running: {err}");
        } else {
            info!("the daemon has finished execution");
        }

        if !watcher_handle.is_finished() {
            watcher_handle.abort();
        }

        if !upstream_poller_handle.is_finished() {
            upstream_poller_handle.abort();
        }

        // TODO: add timeouts and error handling here
        // TODO2: maybe we need to make those fuse futures?
        watcher_handle.await;
        upstream_poller_handle.await;

        Ok(())
    })
}
