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

// #[cfg(target_arch = "wasm32")]
// use wasm_utils::console_log;

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
    // Max: leaving these logs in as they're useful for debugging
    // console_log!("spawn_future: Starting task '{}'", task_name);
    // let task_name_clone = task_name.to_string();

    wasm_bindgen_futures::spawn_local(async move {
        // console_log!("spawn_future: Task '{}' executing", task_name_clone);
        future.await;
        // console_log!("spawn_future: Task '{}' completed", task_name_clone);
    });
    // console_log!("spawn_local returned for task '{}'", task_name);
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

// #[cfg(not(target_arch = "wasm32"))]
// #[track_caller]
// pub fn spawn_named_future<F>(future: F, name: &str)
// where
//     F: Future + Send + 'static,
//     F::Output: Send + 'static,
// {
//     cfg_if::cfg_if! {if #[cfg(tokio_unstable)] {
//         #[allow(clippy::expect_used)]
//         tokio::task::Builder::new().name(name).spawn(future).expect("failed to spawn future");
//     } else {
//         let _ = name;
//         tracing::debug!(r#"the underlying binary hasn't been built with `RUSTFLAGS="--cfg tokio_unstable"` - the future naming won't do anything"#);
//         spawn_future(future);
//     }}
// }

// #[macro_export]
// macro_rules! spawn_future {
//     ($future:expr) => {{
//         $crate::spawn_future($future)
//     }};
//     ($future:expr, $name:expr) => {{
//         cfg_if::cfg_if! {
//             if #[cfg(target_arch = "wasm32")] {
//                 $crate::spawn_future($future, $name)
//             } else {
//                 $crate::spawn_named_future($future, $name)
//             }
//         }
//     }};
// }
