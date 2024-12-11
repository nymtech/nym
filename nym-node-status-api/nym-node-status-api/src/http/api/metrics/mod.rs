use axum::Router;

use crate::http::state::AppState;

pub(crate) mod sessions;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().nest("/sessions", sessions::routes())
    //eventually add other metrics type
}
