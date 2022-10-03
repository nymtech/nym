use std::future::Future;

pub mod client;
pub mod config;
pub mod error;
pub mod init;

// for now we're losing the output but we never really cared about it anyway
pub(crate) fn spawn_future<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    tokio::spawn(future);

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);
}
