// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! smoltcp poll loop for the WASM tunnel.
//!
//! On native, `tokio-smoltcp` drives the smoltcp `Interface` in a background
//! tokio task. On wasm32 we do the same thing with `spawn_local` + `wasmtimer`:
//! a periodic timer fires every 5 ms, polls the interface, and wakes any socket
//! futures whose state has changed.
//!
//! The reactor is notified explicitly (via an mpsc channel) whenever the bridge
//! pushes new rx packets or a socket write completes, so it doesn't rely solely
//! on the timer for responsiveness.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::time::Duration;

use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use smoltcp::iface::{Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp as smoltcp_tcp;
use smoltcp::socket::udp as smoltcp_udp;
use smoltcp::time::Instant;
use wasm_bindgen_futures::spawn_local;

use crate::device::WasmDevice;

/// Poll interval: how often the reactor ticks even without notifications.
///
/// 5 ms matches the typical browser `setInterval` floor and gives smoltcp
/// enough resolution for TCP retransmits and keepalives.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Shared smoltcp network stack, accessed by the reactor, bridge, and sockets.
///
/// Wrapped in `Arc<Mutex<>>` (not `Rc<RefCell<>>`) so that `WasmTunnel` can
/// live in a `OnceLock` which requires `Send + Sync`. On wasm32 (single-threaded),
/// `Mutex` is essentially a no-op lock, zero overhead vs `RefCell`.
pub struct SmoltcpStack {
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub device: WasmDevice,
    pub wakers: HashMap<SocketHandle, SocketWakers>,
}

/// Which smoltcp socket type a handle refers to.
///
/// We track this because `SocketSet::get_mut::<T>(handle)` panics if the
/// type doesn't match (there's no fallible `try_get`).
#[derive(Clone, Copy)]
pub enum SocketKind {
    Tcp,
    Udp,
}

/// Per-socket waker slots + type tag. The reactor checks socket state after
/// each poll and wakes the appropriate future if the socket is ready.
pub struct SocketWakers {
    pub kind: SocketKind,
    pub read: Option<Waker>,
    pub write: Option<Waker>,
    pub connect: Option<Waker>,
}

impl SocketWakers {
    pub fn new(kind: SocketKind) -> Self {
        Self {
            kind,
            read: None,
            write: None,
            connect: None,
        }
    }
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
/// The reactor runs until `shutdown` is set to `true`. It polls the smoltcp
/// interface on:
/// - A periodic 5 ms timer tick
/// - An explicit notification from the bridge or socket write path
///
/// After each poll, it checks all registered sockets and wakes any pending
/// futures whose state has changed (data available, write space, connect done).
pub fn start_reactor(
    stack: Arc<Mutex<SmoltcpStack>>,
    mut notify: mpsc::UnboundedReceiver<()>,
    shutdown: Arc<AtomicBool>,
) {
    spawn_local(async move {
        let mut interval = wasmtimer::tokio::interval(POLL_INTERVAL);

        loop {
            futures::select! {
                _ = interval.tick().fuse() => {},
                _ = notify.next().fuse() => {},
            }

            // Coalesce: drain queued notifications so one poll() handles all
            // state changes. Without this, rapid-fire notifications from TLS
            // writes monopolise the single-threaded WASM event loop.
            while notify.next().now_or_never().flatten().is_some() {}

            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            let mut s = stack.lock().unwrap();
            let now = smoltcp_now();

            let SmoltcpStack {
                ref mut iface,
                ref mut sockets,
                ref mut device,
                ..
            } = *s;
            iface.poll(now, device, sockets);

            wake_ready_sockets(&mut s);
        }
    });
}

/// Check all registered sockets and wake futures whose state has progressed.
fn wake_ready_sockets(stack: &mut SmoltcpStack) {
    let handles: Vec<(SocketHandle, SocketKind)> =
        stack.wakers.iter().map(|(h, w)| (*h, w.kind)).collect();

    for (handle, kind) in handles {
        match kind {
            SocketKind::Tcp => wake_tcp_socket(stack, handle),
            SocketKind::Udp => wake_udp_socket(stack, handle),
        }
    }
}

fn wake_tcp_socket(stack: &mut SmoltcpStack, handle: SocketHandle) {
    let socket = stack.sockets.get_mut::<smoltcp_tcp::Socket>(handle);
    let can_recv = socket.can_recv();
    let can_send = socket.can_send();
    let may_recv = socket.may_recv();
    let state = socket.state();

    let wakers = stack.wakers.get_mut(&handle).unwrap();

    // Wake read waker when data is available OR when no more data will
    // ever arrive. `may_recv()` is false for CloseWait, LastAck, Closed,
    // TimeWait, all states where the remote has sent FIN. Without this,
    // a read waker registered just before FIN arrives never fires and
    // the body.frame() future hangs forever.
    if can_recv || !may_recv {
        if let Some(w) = wakers.read.take() {
            crate::util::debug_log!(
                "[reactor] wake read (can_recv={can_recv}, may_recv={may_recv}, state={state:?})"
            );
            w.wake();
        }
    }
    if can_send {
        if let Some(w) = wakers.write.take() {
            w.wake();
        }
    }
    // TCP connect completes on Established (or CloseWait if peer closed fast)
    if state == smoltcp_tcp::State::Established || state == smoltcp_tcp::State::CloseWait {
        if let Some(w) = wakers.connect.take() {
            w.wake();
        }
    }
    // Wake on terminal states so connect futures don't hang
    if matches!(
        state,
        smoltcp_tcp::State::Closed | smoltcp_tcp::State::TimeWait
    ) {
        if let Some(w) = wakers.connect.take() {
            w.wake();
        }
    }
}

fn wake_udp_socket(stack: &mut SmoltcpStack, handle: SocketHandle) {
    let socket = stack.sockets.get_mut::<smoltcp_udp::Socket>(handle);
    let can_recv = socket.can_recv();
    let can_send = socket.can_send();

    let wakers = stack.wakers.get_mut(&handle).unwrap();

    if can_recv {
        if let Some(w) = wakers.read.take() {
            w.wake();
        }
    }
    if can_send {
        if let Some(w) = wakers.write.take() {
            w.wake();
        }
    }
}
