pub(crate) mod description;
pub(crate) mod hardware;
pub(crate) mod state;
pub(crate) mod stats;
pub(crate) mod verloc;

use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;

pub(crate) async fn not_found(uri: Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("I couldn't find '{uri}'. Try something else?"),
    )
}
