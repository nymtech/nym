use crate::db::DbPool;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    pub(crate) watched_addresses: Vec<String>,
}

impl AppState {
    pub(crate) fn new(db_pool: DbPool, watched_addresses: Vec<String>) -> Self {
        Self {
            db_pool,
            watched_addresses,
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }
}
