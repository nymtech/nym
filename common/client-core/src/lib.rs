use std::future::Future;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
pub mod cli_helpers;
pub mod client;
pub mod config;
pub mod error;
pub mod init;

pub use nym_topology::{
    HardcodedTopologyProvider, NymTopology, NymTopologyError, SerializableNymTopology,
    SerializableTopologyError, TopologyProvider,
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
