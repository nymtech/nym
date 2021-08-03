use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type CountryNodesDistribution = HashMap<String, u32>;

#[derive(Clone)]
pub struct ConcurrentCountryNodesDistribution {
    inner: Arc<RwLock<CountryNodesDistribution>>,
}

impl ConcurrentCountryNodesDistribution {
    pub(crate) fn new() -> Self {
        ConcurrentCountryNodesDistribution {
            inner: Arc::new(RwLock::new(CountryNodesDistribution::new())),
        }
    }

    pub(crate) fn attach(country_node_distribution: CountryNodesDistribution) -> Self {
        ConcurrentCountryNodesDistribution {
            inner: Arc::new(RwLock::new(country_node_distribution)),
        }
    }

    // pub(crate) async fn insert(&mut self, key: String, value: u32) -> Option<u32> {
    //     self.inner.write().await.insert(key, value)
    // }

    // pub(crate) async fn get(&self, key: &str) -> Option<u32> {
    //     self.inner.read().await.get(key).cloned()
    // }

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
