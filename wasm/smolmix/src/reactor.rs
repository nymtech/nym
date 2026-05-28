// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! smoltcp poll loop for the WASM tunnel.
//!
//! Drives `Interface::poll()` in a single `spawn_local` task. The cadence is
//! adaptive: `poll_delay()` reports smoltcp's next soft deadline, the loop
//! sleeps until that deadline (capped by [`MAX_IDLE`]) or until a notification
//! arrives. smoltcp's per-socket `register_recv_waker`/`register_send_waker`
//! fire automatically on every state change.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures::FutureExt;
use nym_wasm_client_core::nym_task::ShutdownTracker;
use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::time::Instant;
use tokio::sync::Notify;
use wasmtimer::std::Instant as MonotonicInstant;

use crate::device::WasmDevice;
use crate::state::{State, TaskName};

/// Maximum idle sleep when smoltcp has no pending work. Bounds the latency of
/// TCP retransmit and keepalive timers if `poll_delay` ever returns `None`; on
/// an active connection the wake source is the bridge or a socket write.
const MAX_IDLE: Duration = Duration::from_secs(60);

/// Shared smoltcp network stack, accessed by the reactor, bridge, and sockets.
///
/// Inner is reached only via [`SmoltcpStack::with`], which scopes lock
/// acquisition to a single closure and prevents callers from holding the
/// guard across `.await` points.
///
/// `Arc<Mutex<>>` so that `WasmTunnel` can live in a
/// `OnceLock` which requires `Send + Sync`. On wasm32 (single-threaded),
/// `Mutex` is essentially a no-op lock, zero overhead vs `RefCell`.
#[derive(Clone)]
pub struct SmoltcpStack {
    inner: Arc<Mutex<SmoltcpStackInner>>,
}

/// Inner state of the smoltcp stack. Only reachable inside a `with` closure.
pub(crate) struct SmoltcpStackInner {
    pub(crate) iface: Interface,
    pub(crate) sockets: SocketSet<'static>,
    pub(crate) device: WasmDevice,
    /// TCP handles awaiting clean removal: their `Drop` queued a FIN via
    /// `socket.close()` but smoltcp hasn't transitioned to `State::Closed`
    /// yet. Swept after each `iface.poll()`.
    pub(crate) pending_removal: Vec<SocketHandle>,
}

impl SmoltcpStack {
    /// Construct a fresh stack around the given interface + device.
    pub fn new(iface: Interface, device: WasmDevice) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SmoltcpStackInner {
                iface,
                sockets: SocketSet::new(Vec::new()),
                device,
                pending_removal: Vec::new(),
            })),
        }
    }

    /// Acquire the lock for a single bounded scope of work.
    ///
    /// The lock is held for the duration of the closure only; callers
    /// physically cannot hold it across `.await` because the closure is
    /// synchronous.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut SmoltcpStackInner) -> R) -> R {
        let mut g = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        f(&mut g)
    }
}

/// Monotonic epoch anchor for `smoltcp_now`. Lazily initialised on first call.
static EPOCH: OnceLock<MonotonicInstant> = OnceLock::new();

/// Yield once to the JS microtask queue, then resume.
///
/// `wasm_bindgen_futures` doesn't expose a `yield_now` directly, so we wake the
/// current task from `poll_fn` to give the executor a chance to process other
/// ready tasks (notify channel, socket wakers) before we re-poll smoltcp.
/// Cheaper than `wasmtimer::sleep(1ms)`, which goes through `setTimeout`
/// (browsers clamp to a ~4ms minimum).
async fn yield_now() {
    let mut yielded = false;
    futures::future::poll_fn(|cx| {
        if yielded {
            std::task::Poll::Ready(())
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .await
}

/// Get the current smoltcp timestamp from a monotonic clock.
///
/// smoltcp's `Instant` is an `i64` of microseconds relative to some epoch.
/// We anchor to the first call and report offsets from that. `wasmtimer::std::Instant`
/// is backed by `performance.now()` on wasm32, which is monotonic within the
/// current Worker agent per the W3C HR-Time spec — unlike `Date::now()`, which
/// can step backwards on NTP correction or user clock changes and would corrupt
/// smoltcp's retransmit/timeout maths.
pub fn smoltcp_now() -> Instant {
    let epoch = *EPOCH.get_or_init(MonotonicInstant::now);
    let elapsed_us = MonotonicInstant::now().duration_since(epoch).as_micros() as i64;
    Instant::from_micros(elapsed_us)
}

/// Wake source for the reactor. Multiple holders call `notify_one()` to ask
/// the reactor to re-poll smoltcp; coalescing is intrinsic to `tokio::sync::Notify`
/// (10 calls before the next iteration are equivalent to 1).
pub type ReactorNotify = Arc<Notify>;

/// Start the smoltcp reactor as a `spawn_local` background task.
///
/// Each iteration:
/// 1. Lock the stack, call `iface.poll()` (which fires socket wakers internally).
/// 2. Ask smoltcp how long it can wait before the next poll (`poll_delay`).
/// 3. Sleep for that duration (capped at [`MAX_IDLE`]) or until a notification.
///
/// Notifications come from the bridge (new rx packets in the device, needing
/// `iface.poll()` to ingest them) and from socket writes (data queued in
/// smoltcp's tx buffer, needing `iface.poll()` to dispatch it to the device).
pub fn start_reactor(
    stack: SmoltcpStack,
    notify: Arc<Notify>,
    tracker: &ShutdownTracker,
    state: State,
) {
    // Cloned so finalise_task can check is_cancelled() on the way out.
    let token = tracker.clone_shutdown_token();
    tracker.try_spawn_named_with_shutdown(
        async move {
            loop {
                // Poll smoltcp; built-in socket wakers fire on any state change.
                let delay = stack.with(|s| {
                    let now = smoltcp_now();
                    let SmoltcpStackInner {
                        iface,
                        sockets,
                        device,
                        pending_removal,
                    } = s;
                    iface.poll(now, device, sockets);

                    // Sweep handles whose FIN/ACK exchange just completed.
                    pending_removal.retain(|&handle| {
                        if sockets.get::<smoltcp_tcp::Socket>(handle).state()
                            == smoltcp_tcp::State::Closed
                        {
                            sockets.remove(handle);
                            false
                        } else {
                            true
                        }
                    });

                    iface.poll_delay(now, sockets)
                });

                // Translate smoltcp's deadline into a wait. A zero delay means
                // "poll again immediately"; yield to the JS event loop via
                // `yield_now()` rather than a 1ms `wasmtimer::sleep`, which
                // schedules a `setTimeout` and is hit by browsers' ~4ms minimum
                // clamp.
                match delay {
                    Some(d) if d.total_micros() == 0 => {
                        yield_now().await;
                    }
                    other => {
                        let sleep_for = match other {
                            Some(d) => Duration::from_micros(d.total_micros()).min(MAX_IDLE),
                            None => MAX_IDLE,
                        };
                        // `Notify` coalesces multiple `notify_one()` calls into
                        // one pending wake, so no drain loop needed. The
                        // explicit `token.cancelled()` arm lets us exit the
                        // loop cleanly into `finalise_task` on shutdown rather
                        // than being aborted mid-select by the supervisor wrapper.
                        futures::select! {
                            _ = wasmtimer::tokio::sleep(sleep_for).fuse() => {},
                            _ = notify.notified().fuse() => {},
                            _ = token.cancelled().fuse() => break,
                        }
                    }
                }
            }

            state.finalise_task(TaskName::Reactor, &token);
        },
        "smolmix-reactor",
    );
}
