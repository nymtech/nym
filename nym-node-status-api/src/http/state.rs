use crate::db::DbPool;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
}

impl AppState {
    pub(crate) fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }
}
