use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type CountryNodesDistribution = HashMap<String, u32>;

#[derive(Clone)]
pub struct ThreadsafeCountryNodesDistribution {
    inner: Arc<RwLock<CountryNodesDistribution>>,
}

impl ThreadsafeCountryNodesDistribution {
    pub(crate) fn new() -> Self {
        ThreadsafeCountryNodesDistribution {
            inner: Arc::new(RwLock::new(CountryNodesDistribution::new())),
        }
    }

    pub(crate) fn new_from_distribution(
        country_node_distribution: CountryNodesDistribution,
    ) -> Self {
        ThreadsafeCountryNodesDistribution {
            inner: Arc::new(RwLock::new(country_node_distribution)),
        }
    }

    pub(crate) async fn set_all(&mut self, country_node_distribution: CountryNodesDistribution) {
        self.inner
            .write()
            .await
            .clone_from(&country_node_distribution)
    }

    pub(crate) async fn get_all(&self) -> HashMap<String, u32> {
        self.inner.read().await.clone()
    }
}
