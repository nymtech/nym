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

#[cfg(target_arch = "wasm32")]
use wasm_utils::console_log;

pub use nym_topology::{
    HardcodedTopologyProvider, NymRouteProvider, NymTopology, NymTopologyError, TopologyProvider,
};

#[deprecated(note = "use spawn_future from nym_task crate instead")]
#[cfg(target_arch = "wasm32")]
#[track_caller]
pub fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + std::panic::UnwindSafe + 'static,
{
    use futures::FutureExt; // MAX might be my RustAnalyzer but when I try and put this under target_arch it moves it out, and the compiler warning was annoying
    console_log!("spawn_future: Starting task '{}'", task_name);

    let wrapped_future = async move {
        console_log!("spawn_future: Task '{}' executing", task_name);

        let result = std::panic::AssertUnwindSafe(future).catch_unwind().await;

        match result {
            Ok(_) => console_log!("spawn_future: Task '{}' completed successfully", task_name),
            Err(_) => console_log!("spawn_future: Task '{}' PANICKED!", task_name),
        }
    };

    wasm_bindgen_futures::spawn_local(wrapped_future);
    console_log!("spawn_local returned for task '{}'", task_name);
}
// #[cfg(target_arch = "wasm32")]
// pub fn spawn_future<F>(future: F)
// where
//     F: Future<Output = ()> + 'static,
// {
//     console_log!("spawn_future called (WASM)");

//     use futures::FutureExt;

//     let wrapped_future = async move {
//         let result = std::panic::AssertUnwindSafe(future).catch_unwind().await;

//         match result {
//             Ok(_) => console_log!("spawn_future: Task completed"),
//             Err(_) => console_log!("spawn_future: Task panicked"),
//         }
//     };

//     wasm_bindgen_futures::spawn_local(wrapped_future);
//     console_log!("spawn_local returned with panic handling");
// }

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
