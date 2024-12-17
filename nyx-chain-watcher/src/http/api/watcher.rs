use crate::config::Config;
use crate::http::error::Error;
use crate::http::error::HttpResult;
use crate::http::state::AppState;
use axum::{Json, Router};

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
    let config =
        Config::read_from_toml_file_in_default_location().map_err(|_| Error::internal())?;

    let addresses = config
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
        .unwrap_or_default();

    Ok(Json(addresses))
}
