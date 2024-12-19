use crate::config::Config;
use crate::env;
use crate::http::error::HttpResult;
use crate::http::state::AppState;
use axum::{Json, Router};
use std::env::var;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/addresses", axum::routing::get(get_addresses))
}

#[utoipa::path(
    tag = "Watcher Configuration",
    get,
    path = "/v1/watcher/addresses",
    responses(
        (status = 200, body = Vec<String>)
    )
)]

/// Fetch the addresses being watched by the chain watcher
async fn get_addresses() -> HttpResult<Json<Vec<String>>> {
    let addresses = match Config::read_from_toml_file_in_default_location() {
        Ok(config) => config
            .payment_watcher_config
            .as_ref()
            .and_then(|config| {
                config.watchers.iter().find_map(|watcher| {
                    watcher
                        .watch_for_transfer_recipient_accounts
                        .as_ref()
                        .map(|accounts| {
                            accounts
                                .iter()
                                .map(|account| account.to_string())
                                .collect::<Vec<_>>()
                        })
                })
            })
            .unwrap_or_default(),
        // If the config file doesn't exist, fall back to env variable
        Err(_) => var(env::vars::NYX_CHAIN_WATCHER_WATCH_ACCOUNTS)
            .map(|accounts| accounts.split(',').map(String::from).collect())
            .unwrap_or_default(),
    };

    Ok(Json(addresses))
}
