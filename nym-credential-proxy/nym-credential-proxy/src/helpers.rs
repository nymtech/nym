// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::attestation_watcher::AttestationWatcher;
use crate::http::state::ApiState;
use crate::{cli::Cli, http::HttpServer};
use nym_bin_common::bin_info;
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::storage::CredentialProxyStorage;
use nym_credential_proxy_lib::ticketbook_manager::TicketbookManager;
use nym_network_defaults::var_names;
use nym_network_defaults::var_names::CONFIGURED;
use tracing::{error, info};

pub async fn wait_for_signal() {
    use tokio::signal::unix::{SignalKind, signal};

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
    let jwt_signing_keys = cli.jwt_signing_keys.signing_keys()?;

    let upgrade_mode_attestation_check_url = match cli.upgrade_mode.attestation_check_url {
        Some(url) => url,
        None => {
            // argument hasn't been provided and env is not configured
            if std::env::var(CONFIGURED).is_err() {
                return Err(CredentialProxyError::AttestationCheckUrlNotSet);
            }
            // argument hasn't been provided and the relevant env value hasn't been set
            // (technically this shouldn't be possible)
            let Ok(env_url) = std::env::var(var_names::UPGRADE_MODE_ATTESTATION_URL) else {
                return Err(CredentialProxyError::AttestationCheckUrlNotSet);
            };

            match env_url.parse() {
                Ok(url) => url,
                Err(err) => {
                    return Err(CredentialProxyError::MalformedAttestationCheckUrl { source: err });
                }
            }
        }
    };

    let attester_pubkey = match cli.upgrade_mode.attester_pubkey {
        Some(pubkey) => pubkey,
        None => {
            // argument hasn't been provided and env is not configured
            if std::env::var(CONFIGURED).is_err() {
                return Err(CredentialProxyError::AttesterPublicKeyNotSet);
            }
            // argument hasn't been provided and the relevant env value hasn't been set
            // (technically this shouldn't be possible)
            let Ok(env_key) = std::env::var(var_names::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY)
            else {
                return Err(CredentialProxyError::AttesterPublicKeyNotSet);
            };

            match env_key.parse() {
                Ok(key) => key,
                Err(err) => {
                    return Err(CredentialProxyError::MalformedAttesterPublicKey { source: err });
                }
            }
        }
    };

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

    let attestation_watcher = AttestationWatcher::new(
        cli.upgrade_mode.attestation_check_regular_polling_interval,
        cli.upgrade_mode
            .attestation_check_expedited_polling_interval,
        attester_pubkey,
        upgrade_mode_attestation_check_url,
        jwt_signing_keys,
        cli.upgrade_mode.upgrade_mode_jwt_validity,
    );

    let api_state = ApiState::new(
        ticketbook_manager.clone(),
        attestation_watcher.shared_state(),
    );

    // spawn the attestation watcher as a separate task
    api_state.try_spawn_in_background(attestation_watcher.run_forever(api_state.shutdown_token()));

    let http_server = HttpServer::new(bind_address, api_state, auth_token);

    // spawn the http server as a separate task / thread(-ish)
    http_server.spawn_as_task();

    // wait for cancel signal (SIGINT, SIGTERM or SIGQUIT)
    wait_for_signal().await;

    // cancel all the tasks and wait for all task to terminate
    ticketbook_manager.cancel_and_wait().await;

    Ok(())
}
