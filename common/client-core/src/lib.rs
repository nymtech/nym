use std::future::Future;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "cli",
    feature = "fs-surb-storage",
    feature = "fs-credentials-storage",
    feature = "fs-gateways-storage"
))]
pub mod cli_helpers;
pub mod client;
pub mod config;
pub mod error;
pub mod init;

pub use nym_topology::{
    HardcodedTopologyProvider, NymRouteProvider, NymTopology, NymTopologyError, TopologyProvider,
};

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future);
}

#[derive(Clone, Default, Debug)]
pub struct ForgetMe {
    client: bool,
    stats: bool,
}

impl ForgetMe {
    pub fn new_all() -> Self {
        Self {
            client: true,
            stats: true,
        }
    }

    pub fn new_client() -> Self {
        Self {
            client: true,
            stats: false,
        }
    }

    pub fn new_stats() -> Self {
        Self {
            client: false,
            stats: true,
        }
    }

    pub fn new(client: bool, stats: bool) -> Self {
        Self { client, stats }
    }

    pub fn any(&self) -> bool {
        self.client || self.stats
    }

    pub fn client(&self) -> bool {
        self.client
    }

    pub fn stats(&self) -> bool {
        self.stats
    }
}
