// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NyxChainWatcherError;
use tokio::join;
use tracing::{error, info, trace};

mod args;
mod config;

use crate::chain_scraper::run_chain_scraper;
use crate::{db, http, payment_listener, price_scraper};
pub(crate) use args::Args;
use nym_task::signal::wait_for_signal;

pub(crate) async fn execute(args: Args, http_port: u16) -> Result<(), NyxChainWatcherError> {
    trace!("passed arguments: {args:#?}");

    let config = config::get_run_config(args)?;

    let db_path = config.database_path();

    info!("Config is {config:#?}");
    info!(
        "Database path is {:?}",
        std::path::Path::new(&db_path)
            .canonicalize()
            .unwrap_or_default()
    );
    info!(
        "Chain History Database path is {:?}",
        std::path::Path::new(&config.chain_scraper_database_path())
            .canonicalize()
            .unwrap_or_default()
    );

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let connection_url = format!("sqlite://{}?mode=rwc", db_path);
    let storage = db::Storage::init(connection_url).await?;
    let watcher_pool = storage.pool_owned().await;

    // Spawn the chain scraper and get its storage

    // Spawn the payment listener task
    let payment_listener_handle = tokio::spawn({
        let price_scraper_pool = storage.pool_owned().await;
        let scraper_pool = storage.pool_owned().await;
        run_chain_scraper(&config, scraper_pool).await?;
        let payment_watcher_config = config.payment_watcher_config.unwrap_or_default();

        async move {
            if let Err(e) =
                payment_listener::run_payment_listener(payment_watcher_config, price_scraper_pool)
                    .await
            {
                error!("Payment listener error: {}", e);
            }
            Ok::<_, anyhow::Error>(())
        }
    });

    // Clone pool for each task that needs it
    //let background_pool = db_pool.clone();

    let price_scraper_handle = tokio::spawn(async move {
        price_scraper::run_price_scraper(&watcher_pool).await;
    });

    let shutdown_handles = http::server::start_http_api(storage.pool_owned().await, http_port)
        .await
        .expect("Failed to start server");

    info!("Started HTTP server on port {}", http_port);

    // Wait for the short-lived tasks to complete
    let _ = join!(price_scraper_handle, payment_listener_handle);

    // Wait for a signal to terminate the long-running task
    wait_for_signal().await;

    if let Err(err) = shutdown_handles.shutdown().await {
        error!("{err}");
    };

    Ok(())
}
