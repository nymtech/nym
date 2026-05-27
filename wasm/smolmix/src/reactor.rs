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

use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use nym_wasm_client_core::nym_task::ShutdownTracker;
use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::time::Instant;
use wasmtimer::std::Instant as MonotonicInstant;

use crate::device::WasmDevice;
use crate::state::{State, TaskName};

/// Maximum idle sleep when smoltcp has no pending work. Bounds the latency of
/// TCP retransmit and keepalive timers if `poll_delay` ever returns `None`; on
/// an active connection the wake source is the bridge or a socket write.
const MAX_IDLE: Duration = Duration::from_secs(60);

/// Shared smoltcp network stack, accessed by the reactor, bridge, and sockets.
///
/// Wrapped in `Arc<Mutex<>>` (not `Rc<RefCell<>>`) so that `WasmTunnel` can
/// live in a `OnceLock` which requires `Send + Sync`. On wasm32 (single-threaded),
/// `Mutex` is essentially a no-op lock, zero overhead vs `RefCell`.
pub struct SmoltcpStack {
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub device: WasmDevice,
    /// TCP handles awaiting clean removal: their `Drop` queued a FIN via
    /// `socket.close()` but smoltcp hasn't transitioned to `State::Closed`
    /// yet. Swept after each `iface.poll()`.
    pub pending_removal: Vec<SocketHandle>,
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

/// Type alias for the channel that notifies the reactor to re-poll.
pub type ReactorNotify = mpsc::UnboundedSender<()>;

/// Start the smoltcp reactor as a `spawn_local` background task.
///
/// Each iteration:
/// 1. Lock the stack, call `iface.poll()` (which fires socket wakers internally).
/// 2. Ask smoltcp how long it can wait before the next poll (`poll_delay`).
/// 3. Sleep for that duration (capped at [`MAX_IDLE`]) or until a notification.
/// 4. Coalesce any further notifications that arrived during the sleep.
///
/// Notifications come from the bridge (new rx packets in the device, needing
/// `iface.poll()` to ingest them) and from socket writes (data queued in
/// smoltcp's tx buffer, needing `iface.poll()` to dispatch it to the device).
pub fn start_reactor(
    stack: Arc<Mutex<SmoltcpStack>>,
    mut notify: mpsc::UnboundedReceiver<()>,
    tracker: &ShutdownTracker,
    state: State,
) {
    // Cloned so finalise_task can check is_cancelled() on the way out.
    let token = tracker.clone_shutdown_token();
    tracker.try_spawn_named_with_shutdown(
        async move {
            loop {
                // Poll smoltcp; built-in socket wakers fire on any state change.
                let delay = {
                    let mut s = stack.lock().unwrap_or_else(|p| p.into_inner());
                    let now = smoltcp_now();
                    let SmoltcpStack {
                        ref mut iface,
                        ref mut sockets,
                        ref mut device,
                        ref mut pending_removal,
                    } = *s;
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
                };

                // Translate smoltcp's deadline into a wait. A zero delay means
                // "poll again immediately"; yield to the JS event loop via
                // `yield_now()` rather than a 1ms `wasmtimer::sleep`, which
                // schedules a `setTimeout` and is hit by browsers' ~4ms minimum
                // clamp. Notify messages arriving during the yield stay queued
                // for the next iteration's `iface.poll()` + select!.
                match delay {
                    Some(d) if d.total_micros() == 0 => {
                        yield_now().await;
                    }
                    other => {
                        let sleep_for = match other {
                            Some(d) => Duration::from_micros(d.total_micros() as u64).min(MAX_IDLE),
                            None => MAX_IDLE,
                        };
                        futures::select! {
                            _ = wasmtimer::tokio::sleep(sleep_for).fuse() => {},
                            msg = notify.next().fuse() => {
                                if msg.is_none() {
                                    // All wake sources gone. If cancellation is
                                    // already in flight, finalise_task no-ops;
                                    // otherwise it marks Failed { TaskExited }.
                                    crate::util::debug_error!(
                                        "[reactor] notify channel closed"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                // Drain queued notifications so one poll() handles all
                // state changes. Without this, rapid-fire notifications from TLS
                // writes monopolise the single-threaded WASM event loop.
                while notify.next().now_or_never().flatten().is_some() {}
            }

            state.finalise_task(TaskName::Reactor, &token);
        },
        "smolmix-reactor",
    );
}
