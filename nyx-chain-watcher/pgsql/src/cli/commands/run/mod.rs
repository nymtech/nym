// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NyxChainWatcherError;
use anyhow::Context;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

mod args;
mod config;

use crate::chain_scraper::run_chain_scraper;
use crate::db::DbPool;
use crate::http::state::{BankScraperModuleState, PaymentListenerState, PriceScraperState};
use crate::payment_listener::PaymentListener;
use crate::price_scraper::PriceScraper;
use crate::{db, http};
pub(crate) use args::Args;
use nym_task::signal::wait_for_signal;

async fn try_insert_watcher_execution_information(
    db_pool: DbPool,
    start: OffsetDateTime,
    end: OffsetDateTime,
    error_message: Option<String>,
) {
    let _ = sqlx::query!(
        r#"
        INSERT INTO watcher_execution(start_ts, end_ts, error_message)
        VALUES ($1, $2, $3)
    "#,
        start.into(),
        end.into(),
        error_message
    )
    .execute(&db_pool)
    .await
    .inspect_err(|err| error!("failed to insert run information: {err}"));
}

async fn wait_for_shutdown(
    db_pool: DbPool,
    start: OffsetDateTime,
    main_cancellation_token: CancellationToken,
    scraper_cancellation_token: CancellationToken,
    mut tasks: JoinSet<Option<anyhow::Result<()>>>,
) {
    async fn finalize_shutdown(
        db_pool: DbPool,
        start: OffsetDateTime,
        main_cancellation_token: CancellationToken,
        scraper_cancellation_token: CancellationToken,
        mut tasks: JoinSet<Option<anyhow::Result<()>>>,
        error_message: Option<String>,
    ) {
        // cancel all tasks
        main_cancellation_token.cancel();
        scraper_cancellation_token.cancel();

        // stupid nasty and hacky workaround to make sure all relevant tasks have finished before hard aborting them
        // nasty stupid and hacky workaround
        tokio::time::sleep(Duration::from_secs(1)).await;
        tasks.abort_all();

        // insert execution result into the db
        try_insert_watcher_execution_information(
            db_pool,
            start,
            OffsetDateTime::now_utc(),
            error_message,
        )
        .await
    }

    tokio::select! {
        // graceful shutdown
        _ = wait_for_signal() => {
            info!("received shutdown signal");
            finalize_shutdown(db_pool, start, main_cancellation_token, scraper_cancellation_token, tasks, None).await;
        }
        _ = scraper_cancellation_token.cancelled() => {
            info!("the scraper has issued cancellation");
            finalize_shutdown(db_pool, start, main_cancellation_token, scraper_cancellation_token, tasks, Some("unexpected scraper task cancellation".into())).await;
        }
        _ = main_cancellation_token.cancelled() => {
            info!("one of the tasks has cancelled the token");
            finalize_shutdown(db_pool, start, main_cancellation_token, scraper_cancellation_token, tasks, Some("unexpected main task cancellation".into())).await;
        }
        task_result = tasks.join_next() => {
            // the first unwrap is fine => join set was not empty
            let error_message = match task_result.unwrap() {
                Err(_join_err) => Some("unexpected join error".to_string()),
                Ok(Some(Ok(_))) => None,
                Ok(Some(Err(err))) => Some(err.to_string()),
                Ok(None) => {
                    Some("unexpected task cancellation".to_string())
                }
            };

            error!("unexpected task termination: {error_message:?}");
            finalize_shutdown(db_pool, start, main_cancellation_token, scraper_cancellation_token, tasks, error_message).await;
        }

    }
}

pub(crate) async fn execute(args: Args, http_port: u16) -> Result<(), NyxChainWatcherError> {
    let start = OffsetDateTime::now_utc();

    info!("passed arguments: {args:#?}");

    let config = config::get_run_config(args)?;

    let db_connection_string = config.chain_scraper_connection_string();

    info!("Config is {config:#?}");
    info!(
        "Chain History Database path is {:?}",
        std::path::Path::new(&config.chain_scraper_connection_string()).canonicalize()
    );

    let storage = db::Storage::init(db_connection_string).await?;
    let watcher_pool = storage.pool_owned();

    let mut tasks = JoinSet::new();
    let cancellation_token = CancellationToken::new();

    let price_scraper_pool = storage.pool_owned();
    let scraper_pool = storage.pool_owned();
    let shutdown_pool = storage.pool_owned();

    // construct shared state
    let payment_listener_shared_state = PaymentListenerState::new();
    let price_scraper_shared_state = PriceScraperState::new();
    let bank_scraper_module_shared_state = BankScraperModuleState::new();

    // spawn all the tasks

    // 1. chain scraper (note: this doesn't really spawn the full scraper on this task, but we don't want to be blocking waiting for its startup)
    let scraper_token_handle: JoinHandle<anyhow::Result<CancellationToken>> = tokio::spawn({
        let config = config.clone();
        let shared_state = bank_scraper_module_shared_state.clone();
        async move {
            // this only blocks until startup sync is done; it then runs on its own set of tasks
            let scraper = run_chain_scraper(&config, scraper_pool, shared_state).await?;
            Ok(scraper.cancel_token())
        }
    });

    // 2. payment listener
    let token = cancellation_token.clone();
    let payment_watcher_config = config.payment_watcher_config.clone();
    let payment_listener = PaymentListener::new(
        price_scraper_pool,
        payment_watcher_config,
        payment_listener_shared_state.clone(),
    )?;
    {
        tasks.spawn(async move {
            token
                .run_until_cancelled(async move {
                    payment_listener.run().await;
                    Ok(())
                })
                .await
        });
    }

    // 3. price scraper (note, this task never terminates on its own)
    let price_scraper = PriceScraper::new(price_scraper_shared_state.clone(), watcher_pool);
    {
        let token = cancellation_token.clone();
        tasks.spawn(async move {
            token
                .run_until_cancelled(async move {
                    price_scraper.run().await;
                    Ok(())
                })
                .await
        });
    }

    // 4. http api
    let http_server = http::server::build_http_api(
        storage.pool_owned(),
        &config,
        http_port,
        payment_listener_shared_state,
        price_scraper_shared_state,
        bank_scraper_module_shared_state,
    )
    .await?;
    {
        let token = cancellation_token.clone();
        tasks.spawn(async move {
            info!("Starting HTTP server on port {http_port}",);
            async move {
                Some(
                    http_server
                        .run(token.cancelled_owned())
                        .await
                        .context("http server failure"),
                )
            }
            .await
        });
    }

    // 1. wait for either shutdown or scraper having finished startup
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
