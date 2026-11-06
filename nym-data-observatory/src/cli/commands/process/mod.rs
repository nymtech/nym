// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::chain_scraper::process_chain_scraper;
pub(crate) use crate::cli::commands::process::args::Args;
use crate::cli::commands::run::wait_for_shutdown;
use crate::db;
use crate::error::NymDataObservatoryError;
use nym_task::wait_for_signal;
use time::OffsetDateTime;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

mod args;

pub(crate) async fn execute(args: Args) -> Result<(), NymDataObservatoryError> {
    let start = OffsetDateTime::now_utc();
    let scraper_args = args.clone();

    let run_args = crate::cli::commands::run::args::Args {
        rpc_url: args.rpc_url.clone(),
        websocket_url: args.websocket_url.clone(),
        start_block_height: Some(args.start_block_height),
        db_connection_string: args.db_connection_string,
        webhook_url: args.webhook_url.clone(),
        watch_for_chain_message_types: args.watch_for_chain_message_types,
        webhook_auth: args.webhook_auth.clone(),
    };

    let config = crate::cli::commands::run::config::get_run_config(run_args.clone())?;

    let db_connection_string = config.chain_scraper_connection_string();

    let start_block_height = args.start_block_height;
    let end_block_height = args
        .end_block_height
        .unwrap_or(args.start_block_height + (args.blocks_to_process.unwrap_or(1u32) - 1u32));

    info!("nyxd rpc: {}", args.rpc_url.to_string());
    info!("start_block_height: {:#?}", start_block_height);
    info!("end_block_height: {:#?}", end_block_height);
    info!("blocks_to_process: {:#?}", args.blocks_to_process);

    let storage = db::Storage::init(db_connection_string).await?;

    let tasks = JoinSet::new();
    let cancellation_token = CancellationToken::new();

    let scraper_pool = storage.pool_owned();
    let shutdown_pool = storage.pool_owned();

    // start the blocks processing in the background, that can be cancelled by the user
    let cancel_after_processing = cancellation_token.clone();
    let scraper_token_handle: JoinHandle<anyhow::Result<CancellationToken>> = tokio::spawn({
        let config = config.clone();
        async move {
            // this only blocks until startup sync is done; it then runs on its own set of tasks
            let scraper = process_chain_scraper(
                scraper_args,
                &config,
                scraper_pool,
                start_block_height,
                end_block_height,
            )
            .await?;

            info!("⏰ shutting down...");
            cancel_after_processing.cancel();

            Ok(scraper.cancel_token())
        }
    });

    // wait for either shutdown or scraper having finished processing the block range
    tokio::select! {
        _ = wait_for_signal() => {
            info!("received shutdown signal while waiting for scraper to finish its startup");
            return Ok(())
        }
        scraper_token = scraper_token_handle => {
            let scraper_token = match scraper_token {
                Ok(Ok(token)) => token,
                Ok(Err(startup_err)) => {
                    error!("failed to startup the chain scraper: {startup_err}");
                    return Err(startup_err.into());
                }
                Err(runtime_err) => {
                    error!("failed to finish the scraper startup task: {runtime_err}");
                    return Ok(())

                }
            };

            wait_for_shutdown(shutdown_pool, start, cancellation_token, scraper_token, tasks).await
        }
    }

    Ok(())
}
