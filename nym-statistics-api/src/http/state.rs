use crate::storage::StatisticsStorage;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    storage_manager: StatisticsStorage,
}

impl AppState {
    pub(crate) async fn new(storage_manager: StatisticsStorage) -> Self {
        Self { storage_manager }
    }

    pub(crate) fn storage(&mut self) -> &mut StatisticsStorage {
        &mut self.storage_manager
    }
}
