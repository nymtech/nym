use axum::Router;

use crate::http::state::AppState;

pub(crate) mod sessions;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().nest("/sessions", sessions::routes())
    //eventually add other metrics type
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_routes_construction() {
        let router = routes();
        // Verify the router builds without panic
        let _routes = router;
    }
}
