use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
pub type JoinHandle<F> = tokio::task::JoinHandle<F>;

// no JoinHandle equivalent in wasm

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub struct FakeJoinHandle<F> {
    _p: std::marker::PhantomData<F>,
}
#[cfg(target_arch = "wasm32")]
pub type JoinHandle<F> = FakeJoinHandle<F>;

#[cfg(target_arch = "wasm32")]
#[track_caller]
pub fn spawn_future<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        // make sure the future outputs `()`
        future.await;
    });
    FakeJoinHandle {
        _p: std::marker::PhantomData,
    }
}

// Note: prefer spawning tasks directly on the ShutdownManager
#[cfg(not(target_arch = "wasm32"))]
#[track_caller]
pub fn spawn_future<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

// Note: prefer spawning tasks directly on the ShutdownManager
#[cfg(not(target_arch = "wasm32"))]
#[track_caller]
pub fn spawn_named_future<F>(future: F, name: &str) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    cfg_if::cfg_if! {if #[cfg(all(tokio_unstable, feature="tokio-tracing"))] {
        #[allow(clippy::expect_used)]
        tokio::task::Builder::new().name(name).spawn(future).expect("failed to spawn future")
    } else {
        let _ = name;
        tracing::debug!(r#"the underlying binary hasn't been built with `RUSTFLAGS="--cfg tokio_unstable"` - the future naming won't do anything"#);
        spawn_future(future)
    }}
}

#[cfg(target_arch = "wasm32")]
#[track_caller]
pub fn spawn_named_future<F>(future: F, name: &str) -> JoinHandle<F::Output>
where
    F: Future + 'static,
{
    // not supported in wasm
    let _ = name;
    spawn_future(future)
}

#[macro_export]
macro_rules! spawn_future {
    ($future:expr) => {{
        $crate::spawn_future($future)
    }};
    ($future:expr, $name:expr) => {{
        $crate::spawn_named_future($future, $name)
    }};
}
