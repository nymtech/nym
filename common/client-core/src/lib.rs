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
use wasm_utils::console_log;

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
    console_log!("spawn_future called (WASM)");
    wasm_bindgen_futures::spawn_local(future);
    console_log!("spawn_local returned");
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
