#![allow(deprecated)] // silences clippy warning: use of deprecated associated function `nym_crypto::generic_array::GenericArray::<T, N>::clone_from_slice`: please upgrade to generic-array 1.x - TODO
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

#[deprecated(note = "use spawn_future from nym_task crate instead")]
#[cfg(target_arch = "wasm32")]
#[track_caller]
pub fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        future.await;
    });
}

#[deprecated(note = "use spawn_future from nym_task crate instead")]
#[cfg(not(target_arch = "wasm32"))]
#[track_caller]
pub fn spawn_future<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future);
}
