use std::future::Future;

pub mod client;
pub mod config;
pub mod error;
pub mod init;
pub mod temp;

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
