// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::bin_info;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::deposits_buffer::DepositsBuffer;
use crate::http::state::required_deposit_cache::RequiredDepositCache;
use crate::quorum_checker::QuorumStateChecker;
use crate::{
    cli::Cli,
    error::CredentialProxyError,
    http::{
        state::{ApiState, ChainClient},
        HttpServer,
    },
    storage::CredentialProxyStorage,
    tasks::StoragePruner,
};

pub struct LockTimer {
    created: OffsetDateTime,
    message: String,
}

impl LockTimer {
    pub fn new<S: Into<String>>(message: S) -> Self {
        LockTimer {
            message: message.into(),
            ..Default::default()
        }
    }
}

impl Drop for LockTimer {
    fn drop(&mut self) {
        let time_taken = OffsetDateTime::now_utc() - self.created;
        let time_taken_formatted = humantime::format_duration(time_taken.unsigned_abs());
        if time_taken > time::Duration::SECOND * 10 {
            warn!(time_taken = %time_taken_formatted, "{}", self.message)
        } else if time_taken > time::Duration::SECOND * 5 {
            info!(time_taken = %time_taken_formatted, "{}", self.message)
        } else {
            debug!(time_taken = %time_taken_formatted, "{}", self.message)
        };
    }
}

impl Default for LockTimer {
    fn default() -> Self {
        LockTimer {
            created: OffsetDateTime::now_utc(),
            message: "released the lock".to_string(),
        }
    }
}

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
    // create the tasks
    let bind_address = cli.bind_address();

    let storage = CredentialProxyStorage::init(cli.persistent_storage_path()).await?;
    let mnemonic = cli.mnemonic;
    let auth_token = cli.http_auth_token;
    let webhook_cfg = cli.webhook;
    let chain_client = ChainClient::new(mnemonic)?;
    let cancellation_token = CancellationToken::new();

    let required_deposit_cache = RequiredDepositCache::default();

    let quorum_state_checker = QuorumStateChecker::new(
        chain_client.clone(),
        cli.quorum_check_interval,
        cancellation_token.clone(),
    )
    .await?;
    let quorum_state = quorum_state_checker.quorum_state_ref();

    let deposits_buffer = DepositsBuffer::new(
        storage.clone(),
        chain_client.clone(),
        required_deposit_cache.clone(),
        build_sha_short(),
        cli.deposits_buffer_size,
        cli.max_concurrent_deposits,
        cancellation_token.clone(),
    )
    .await?;

    // let deposit_request_sender = deposit_maker.deposit_request_sender();
    let api_state = ApiState::new(
        storage.clone(),
        quorum_state,
        webhook_cfg,
        chain_client,
        deposits_buffer,
        required_deposit_cache,
        cancellation_token.clone(),
    )
    .await?;
    let http_server = HttpServer::new(
        bind_address,
        api_state.clone(),
        auth_token,
        cancellation_token.clone(),
    );
    let storage_pruner = StoragePruner::new(cancellation_token, storage);

    // spawn all the tasks
    api_state.try_spawn(http_server.run_forever());
    api_state.try_spawn(storage_pruner.run_forever());
    api_state.try_spawn(quorum_state_checker.run_forever());

    // wait for cancel signal (SIGINT, SIGTERM or SIGQUIT)
    wait_for_signal().await;

    // cancel all the tasks and wait for all task to terminate
    api_state.cancel_and_wait().await;

    Ok(())
}
