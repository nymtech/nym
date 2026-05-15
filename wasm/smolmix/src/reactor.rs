// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! smoltcp poll loop for the WASM tunnel.
//!
//! Drives `Interface::poll()` in a single `spawn_local` task. The cadence is
//! adaptive: `poll_delay()` reports smoltcp's next soft deadline, the loop
//! sleeps until that deadline (capped by [`MAX_IDLE`]) or until a notification
//! arrives. smoltcp's per-socket `register_recv_waker`/`register_send_waker`
//! fire automatically on every state change.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use smoltcp::iface::{Interface, SocketSet};
use smoltcp::time::Instant;
use wasm_bindgen_futures::spawn_local;

use crate::device::WasmDevice;

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
}

/// Get the current smoltcp timestamp from `Date.now()`.
///
/// smoltcp's `Instant` is just `i64` microseconds, not `std::time::Instant`.
/// We convert JS milliseconds (f64) to microseconds (i64).
pub fn smoltcp_now() -> Instant {
    Instant::from_micros((js_sys::Date::now() * 1000.0) as i64)
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
    shutdown: Arc<AtomicBool>,
) {
    spawn_local(async move {
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Poll smoltcp; built-in socket wakers fire on any state change.
            let delay = {
                let mut s = stack.lock().unwrap();
                let now = smoltcp_now();
                let SmoltcpStack {
                    ref mut iface,
                    ref mut sockets,
                    ref mut device,
                } = *s;
                iface.poll(now, device, sockets);
                iface.poll_delay(now, sockets)
            };

            // Translate smoltcp's deadline into a sleep duration. A zero delay
            // means "poll again immediately"; treat it as 1 ms so the task
            // yields back to the JS event loop before re-entering the loop.
            let sleep_for = match delay {
                Some(d) if d.total_micros() == 0 => Duration::from_millis(1),
                Some(d) => Duration::from_micros(d.total_micros() as u64).min(MAX_IDLE),
                None => MAX_IDLE,
            };

            futures::select! {
                _ = wasmtimer::tokio::sleep(sleep_for).fuse() => {},
                _ = notify.next().fuse() => {},
            }

            // Coalesce: drain queued notifications so one poll() handles all
            // state changes. Without this, rapid-fire notifications from TLS
            // writes monopolise the single-threaded WASM event loop.
            while notify.next().now_or_never().flatten().is_some() {}
        }
    });
}
