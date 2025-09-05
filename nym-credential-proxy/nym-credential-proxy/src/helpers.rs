// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{cli::Cli, http::HttpServer};
use nym_bin_common::bin_info;
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::storage::CredentialProxyStorage;
use nym_credential_proxy_lib::ticketbook_manager::TicketbookManager;
use tracing::{error, info};

pub async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    // if we fail to setup the signals, we should just blow up
    #[allow(clippy::expect_used)]
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM channel");
    #[allow(clippy::expect_used)]
    let mut sigquit = signal(SignalKind::quit()).expect("Failed to setup SIGQUIT channel");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received SIGINT");
        },
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        }
        _ = sigquit.recv() => {
            info!("Received SIGQUIT");
        }
    }
}

#[allow(clippy::panic)]
fn build_sha_short() -> &'static str {
    let bin_info = bin_info!();
    if bin_info.commit_sha.len() < 7 {
        panic!("unavailable build commit sha")
    }

    if bin_info.commit_sha == "VERGEN_IDEMPOTENT_OUTPUT" {
        error!("the binary hasn't been built correctly. it doesn't have a commit sha information");
        return "unknown";
    }

    &bin_info.commit_sha[..7]
}

pub(crate) async fn run_api(cli: Cli) -> Result<(), CredentialProxyError> {
    let bind_address = cli.bind_address();
    let storage = CredentialProxyStorage::init(cli.persistent_storage_path()).await?;
    let mnemonic = cli.mnemonic;
    let auth_token = cli.http_auth_token;
    let webhook_cfg = cli.webhook;

    let ticketbook_manager = TicketbookManager::new(
        build_sha_short(),
        cli.quorum_check_interval,
        cli.deposits_buffer_size,
        cli.max_concurrent_deposits,
        storage,
        mnemonic,
        webhook_cfg.try_into()?,
    )
    .await?;

    let http_server = HttpServer::new(bind_address, ticketbook_manager.clone(), auth_token);

    // spawn the http server as a separate task / thread(-ish)
    http_server.spawn_as_task();

    // wait for cancel signal (SIGINT, SIGTERM or SIGQUIT)
    wait_for_signal().await;

    // cancel all the tasks and wait for all task to terminate
    ticketbook_manager.cancel_and_wait().await;

    Ok(())
}
