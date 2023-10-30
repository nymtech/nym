use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
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
