// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_load_current_config;
use crate::daemon::Daemon;
use crate::env::Env;
use crate::error::NymvisorError;
use async_file_watcher::AsyncFileWatcher;
use futures::channel::mpsc;
use futures::future::{AbortHandle, Abortable};
use futures::StreamExt;
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

    // we have three tasks only:
    // - one for managing the daemon launcher
    // - the other one for watching the upgrade plan file
    // - the last one for polling upstream source for upgrade info
    // so once the daemon has finished, for whatever reason, abort the file watcher and upstream poller and terminate the nymvisor

    todo!()
    // // spawn the root task
    // rt.block_on(async {
    //     println!("run");
    //
    //     let daemon = Daemon::from_config(&config);
    //     let running = daemon.execute_async(args.daemon_args)?;
    //
    //     let handle1 = tokio::spawn(async move {
    //         let res = running.await;
    //         println!("the process has finished! with {res:?}");
    //     });
    //
    //     let (events_sender, mut events_receiver) = mpsc::unbounded();
    //     let mut watcher = AsyncFileWatcher::new_file_changes_watcher(
    //         config.upgrade_plan_filepath(),
    //         events_sender,
    //     )?;
    //
    //     let (abort_handle, abort_registration) = AbortHandle::new_pair();
    //
    //     let handle2 =
    //         tokio::spawn(async move { Abortable::new(watcher.watch(), abort_registration).await });
    //
    //     let event = events_receiver.next().await;
    //     println!("watcher event: {event:?}");
    //     interrupt_notify.notify_one();
    //
    //     handle1.await;
    //     abort_handle.abort();
    //     handle2.await;
    //
    //     // println!("{:?}", status);
    //
    //     <Result<_, NymvisorError>>::Ok(())
    // })
}
