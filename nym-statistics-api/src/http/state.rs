use crate::{network_view::NetworkView, storage::StatisticsStorage};

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    storage_manager: StatisticsStorage,
    network_view: NetworkView,
}

impl AppState {
    pub(crate) async fn new(storage_manager: StatisticsStorage, network_view: NetworkView) -> Self {
        Self {
            storage_manager,
            network_view,
        }
    }

    pub(crate) fn storage(&mut self) -> &mut StatisticsStorage {
        &mut self.storage_manager
    }

    pub(crate) fn network_view(&self) -> &NetworkView {
        &self.network_view
    }
}
