use crate::db::DbPool;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    cache: HttpCache,
}

impl AppState {
    pub(crate) fn new(db_pool: DbPool) -> Self {
        Self {
            db_pool,
            cache: HttpCache::new(),
        }
    }

    pub(crate) fn cache(&self) -> &HttpCache {
        &self.cache
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpCache {}

impl HttpCache {
    pub(crate) fn new() -> Self {
        // TODO dz
        HttpCache {}
    }
}
