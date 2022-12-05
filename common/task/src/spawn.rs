use crate::ShutdownListener;
use std::future::Future;

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future);
}

pub fn spawn_with_report_error<F, T, E>(future: F, mut shutdown: ShutdownListener)
where
    F: Future<Output = Result<T, E>> + Send + 'static,
    T: 'static,
    E: std::error::Error + Send + 'static,
{
    let future_that_sends = async move {
        if let Err(err) = future.await {
            shutdown.send_we_stopped(Box::new(err));
        }
    };
    spawn(future_that_sends);
}
