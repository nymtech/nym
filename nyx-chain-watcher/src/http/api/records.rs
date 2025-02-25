use crate::db::models::PaymentRecord;
use crate::db::queries::payments::get_transaction_record;
use crate::http::error::{Error, HttpResult};
use crate::http::state::AppState;
use axum::{extract::{State, Path}, Json, Router};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/records/:record_txs", axum::routing::get(transaction_record))
}

#[utoipa::path(
    tag = "Watcher Records",
    get,
    path = "/v1/records/{record_txs}",
    responses(
        (status = 200, body = PaymentRecord),
        (status = 404, description = "Transaction record not found")
    )
)]
/// Fetch a transaction record from the database
async fn transaction_record(
    State(state): State<AppState>,
    Path(record_txs): Path<String>,
) -> HttpResult<Json<PaymentRecord>> {
    get_transaction_record(state.db_pool(), &record_txs)
        .await
        .map_err(|_| Error::internal())?
        .map(Json::from)
        .ok_or_else(|| Error::not_found(&record_txs))
}
