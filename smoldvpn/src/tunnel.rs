// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The tunnel: builder, lifecycle, and the datapath task.
//!
//! A [`Tunnel`] owns a smol-core [`Stack`] (the tokio socket surface) and spawns
//! a single datapath task that shuttles packets between the stack and the active
//! WireGuard transport, encrypting/decrypting via the [`WgEngine`]. The task is
//! the sole owner of the `Tunn`s (no locking) and is driven by a `select!` over:
//! outbound app packets, inbound WireGuard packets, a boringtun timer tick, and
//! the `CancellationToken`.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures::channel::mpsc;
use futures::StreamExt;
use nym_smol_core::{ChannelDevice, DnsConfig, Stack, StackConfig, TcpStream, UdpSocket};

use crate::connectors::TunnelConnector;
use crate::topup::{
    event_channel, run_topup, BandwidthCredentialSource, BandwidthEvent, TopupConfig,
};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::bridge::{self, BridgeParams};
use crate::config::{DnsMode, MtuConfig, PeerConfig, TunnelConfig};
use crate::engine::WgEngine;
use crate::error::{DvpnError, Result};
use crate::transport::{direct_transport, SocketProtector, WgReceiver, WgSender};

/// boringtun timer pump interval.
const TIMER_INTERVAL: Duration = Duration::from_millis(250);

/// Backoff after a transient (Direct UDP) transport recv error, so a persistently failing socket
/// does not busy-loop the datapath.
const TRANSPORT_ERROR_BACKOFF: Duration = Duration::from_millis(100);

/// A shared, swappable handle to the current smol-core stack. Swapped in place
/// when the MTU changes at runtime (see [`Tunnel::set_mtu`]).
type SharedStack = Arc<RwLock<Arc<Stack>>>;

/// The stack-side datapath channels handed to the datapath task: the receiver
/// of app packets from the stack, and the sender of decrypted packets into it.
type StackChannels = (
    mpsc::UnboundedReceiver<Vec<u8>>,
    mpsc::UnboundedSender<Vec<u8>>,
);

/// Build a fresh smol-core stack + its datapath channels for the given MTU.
fn build_stack(
    assigned: Ipv4Addr,
    ipv6: Option<Ipv6Addr>,
    dns: DnsMode,
    interface_mtu: usize,
) -> (Stack, StackChannels) {
    // stack_out: stack -> datapath (app IP packets to encrypt)
    // stack_in:  datapath -> stack (decrypted IP packets)
    let (stack_out_tx, stack_out_rx) = mpsc::unbounded::<Vec<u8>>();
    let (stack_in_tx, stack_in_rx) = mpsc::unbounded::<Vec<u8>>();

    // The device is the single source of truth for the interface MTU.
    let device = ChannelDevice::new(stack_in_rx, stack_out_tx, Some(interface_mtu));
    let mut stack_config = StackConfig::new(assigned);
    if let Some(v6) = ipv6 {
        stack_config = stack_config.with_ipv6(v6);
    }
    let mut stack = Stack::new(device, stack_config);
    if let DnsMode::InTunnelServer(server) = dns {
        stack = stack.with_dns_config(DnsConfig {
            server,
            ..DnsConfig::default()
        });
    }
    (stack, (stack_out_rx, stack_in_tx))
}

/// Which data-plane transport to use.
#[derive(Clone, Debug)]
enum TransportChoice {
    /// Real UDP datagrams to the entry gateway.
    Direct,
    /// QUIC bridge fronting the two-hop entry leg.
    Quic(BridgeParams),
}

/// Bandwidth monitoring/top-up settings for the tunnel: the endpoint + thresholds to poll, and an
/// optional credential source. With a source, the tunnel tops up automatically; without one it only
/// emits [`BandwidthEvent`]s so the caller can react.
struct TopupSpec {
    config: TopupConfig,
    source: Option<Arc<dyn BandwidthCredentialSource>>,
}

/// Builder for a [`Tunnel`].
pub struct TunnelBuilder {
    entry: PeerConfig,
    exit: Option<PeerConfig>,
    config: TunnelConfig,
    transport: TransportChoice,
    protector: Option<SocketProtector>,
    cancel: Option<CancellationToken>,
    topup: Option<TopupSpec>,
}

impl TunnelBuilder {
    /// Start a single-hop tunnel to one gateway.
    pub fn single_hop(gateway: PeerConfig) -> Self {
        Self {
            entry: gateway,
            exit: None,
            config: TunnelConfig::default(),
            transport: TransportChoice::Direct,
            protector: None,
            cancel: None,
            topup: None,
        }
    }

    /// Start a two-hop tunnel through `entry` to `exit`.
    pub fn two_hop(entry: PeerConfig, exit: PeerConfig) -> Self {
        Self {
            entry,
            exit: Some(exit),
            config: TunnelConfig::default(),
            transport: TransportChoice::Direct,
            protector: None,
            cancel: None,
            topup: None,
        }
    }

    /// Override tunnel configuration (MTU, DNS, exit client port).
    pub fn config(mut self, config: TunnelConfig) -> Self {
        self.config = config;
        self
    }

    /// Route the WireGuard data plane over a QUIC bridge (two-hop only).
    pub fn quic_bridge(mut self, params: BridgeParams) -> Self {
        self.transport = TransportChoice::Quic(params);
        self
    }

    /// Install a socket-protection callback (Linux/Android).
    pub fn socket_protector(mut self, protector: SocketProtector) -> Self {
        self.protector = Some(protector);
        self
    }

    /// Provide a cancellation token to abort setup or tear down the tunnel.
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// Run a background bandwidth top-up task: poll the gateway `metadata`
    /// endpoint **through the tunnel** and spend stored tickets (via `source`)
    /// before the registered bandwidth is exhausted. Also emits
    /// [`BandwidthEvent`]s (see [`Tunnel::bandwidth_events`]). Stops with the tunnel.
    pub fn bandwidth_topup(
        mut self,
        config: TopupConfig,
        source: Arc<dyn BandwidthCredentialSource>,
    ) -> Self {
        self.topup = Some(TopupSpec {
            config,
            source: Some(source),
        });
        self
    }

    /// Monitor bandwidth without automatic top-up: poll the gateway `metadata`
    /// endpoint through the tunnel and emit [`BandwidthEvent`]s (see
    /// [`Tunnel::bandwidth_events`]) so the caller can react (e.g. prompt the user
    /// to buy more ticketbooks). No tickets are ever spent.
    pub fn bandwidth_monitor(mut self, config: TopupConfig) -> Self {
        self.topup = Some(TopupSpec {
            config,
            source: None,
        });
        self
    }

    /// Bring up the tunnel.
    pub async fn connect(self) -> Result<Tunnel> {
        Tunnel::connect(self).await
    }
}

/// The tunnel's WireGuard session(s) did not establish within the caller's
/// bound. Reports per-hop status so a caller reusing cached registrations can
/// invalidate (and re-register) exactly the failed hop(s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotEstablished {
    /// Whether the entry hop's WireGuard session established.
    pub entry: bool,
    /// Whether the exit hop's session established; `None` for single-hop tunnels.
    pub exit: Option<bool>,
}

impl std::fmt::Display for NotEstablished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.exit {
            None => write!(f, "wireguard session not established"),
            Some(exit) => write!(
                f,
                "wireguard session(s) not established (entry: {}, exit: {})",
                if self.entry { "up" } else { "down" },
                if exit { "up" } else { "down" },
            ),
        }
    }
}

impl std::error::Error for NotEstablished {}

/// A running dVPN tunnel exposing tokio socket surfaces.
pub struct Tunnel {
    stack: SharedStack,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
    topup_task: Option<JoinHandle<()>>,
    // Parameters needed to rebuild the stack on a runtime MTU change.
    assigned: Ipv4Addr,
    ipv6: Option<Ipv6Addr>,
    dns: DnsMode,
    two_hop: bool,
    mtu: RwLock<MtuConfig>,
    // Hands the datapath the new stack's channels when the stack is swapped.
    swap_tx: mpsc::UnboundedSender<StackChannels>,
    // Serialises `set_mtu` so a concurrent pair can't leave the published stack and the datapath's
    // channels referring to different stacks.
    swap_lock: std::sync::Mutex<()>,
    // Broadcasts bandwidth monitor/top-up events to subscribers.
    events_tx: broadcast::Sender<BandwidthEvent>,
    // Per-hop establishment state published by the datapath (entry, exit).
    established_rx: watch::Receiver<(bool, Option<bool>)>,
}

impl Tunnel {
    async fn connect(builder: TunnelBuilder) -> Result<Tunnel> {
        let cancel = builder.cancel.unwrap_or_default();
        let two_hop = builder.exit.is_some();

        // QUIC bridging only fronts the two-hop entry leg.
        if matches!(builder.transport, TransportChoice::Quic(_)) && !two_hop {
            return Err(DvpnError::QuicRequiresTwoHop);
        }

        // The application-visible interface lives in the exit hop's IP space for
        // two-hop, or the single gateway's for one-hop. MTU follows suit.
        let (assigned, ipv6, interface_mtu) = if let Some(exit) = &builder.exit {
            (
                exit.assigned_ipv4,
                exit.assigned_ipv6,
                builder.config.mtu.exit,
            )
        } else {
            (
                builder.entry.assigned_ipv4,
                builder.entry.assigned_ipv6,
                builder.config.mtu.entry,
            )
        };

        if cancel.is_cancelled() {
            return Err(DvpnError::Cancelled);
        }

        // Build the initial stack + its datapath channels.
        let (stack, (stack_out_rx, stack_in_tx)) =
            build_stack(assigned, ipv6, builder.config.dns, interface_mtu);

        // Control channel to hand the datapath a rebuilt stack's channels on a
        // runtime MTU change (the WireGuard engine/session is preserved).
        let (swap_tx, swap_rx) = mpsc::unbounded::<StackChannels>();

        // Build the engine.
        let engine = if let Some(exit) = &builder.exit {
            let exit_endpoint = as_socket_addr_v4(exit.endpoint, "exit gateway endpoint")?;
            // The inner exit->gateway frame travels through the ENTRY tunnel, so
            // its source must be the entry-assigned tunnel IP — the entry
            // gateway's cryptokey routing (allowed-ips) drops any other source.
            let tunnel_src =
                SocketAddrV4::new(builder.entry.assigned_ipv4, builder.config.exit_client_port);
            WgEngine::two_hop(&builder.entry, exit, tunnel_src, exit_endpoint)
        } else {
            WgEngine::single_hop(&builder.entry)
        };

        // Build the transport (Direct UDP or QUIC bridge).
        let (sender, receiver) = match &builder.transport {
            TransportChoice::Direct => {
                direct_transport(builder.entry.endpoint, builder.protector.as_ref()).await?
            }
            TransportChoice::Quic(params) => {
                let (s, r) = bridge::connect(params, &cancel).await?;
                (WgSender::Quic(s), WgReceiver::Quic(r))
            }
        };

        info!(
            two_hop,
            quic = matches!(builder.transport, TransportChoice::Quic(_)),
            assigned = %assigned,
            mtu = interface_mtu,
            entry_endpoint = %builder.entry.endpoint,
            exit_endpoint = builder.exit.as_ref().map(|e| e.endpoint.to_string()),
            "dVPN tunnel datapath starting"
        );

        // Per-hop establishment state: written by the datapath from the engine's
        // handshake tracking, awaited via `Tunnel::await_established`.
        let (established_tx, established_rx) = watch::channel((false, two_hop.then_some(false)));

        let task = tokio::spawn(datapath(
            engine,
            sender,
            receiver,
            (stack_out_rx, stack_in_tx),
            swap_rx,
            established_tx,
            cancel.clone(),
        ));

        let shared_stack: SharedStack = Arc::new(RwLock::new(Arc::new(stack)));

        // Bandwidth events are always available (empty until a monitor/top-up runs).
        let events_tx = event_channel();

        // Optional background bandwidth monitor/top-up task, dialling the metadata
        // endpoint through this tunnel's connector.
        let topup_task = builder.topup.map(|spec| {
            let connector = TunnelConnector::new(shared_stack.clone());
            tokio::spawn(run_topup(
                spec.config,
                spec.source,
                connector,
                events_tx.clone(),
                cancel.clone(),
            ))
        });

        Ok(Tunnel {
            stack: shared_stack,
            cancel,
            task: Some(task),
            topup_task,
            assigned,
            ipv6,
            dns: builder.config.dns,
            two_hop,
            mtu: RwLock::new(builder.config.mtu),
            swap_tx,
            swap_lock: std::sync::Mutex::new(()),
            events_tx,
            established_rx,
        })
    }

    /// Wait until every WireGuard session this tunnel needs (entry, plus exit
    /// for two-hop) has established, bounded by `timeout`.
    ///
    /// On timeout the error reports per-hop status — the caller can invalidate
    /// exactly the failed hop's cached registration (see
    /// `Session::invalidate_registration` in `nym-sdk-session`) and register
    /// fresh. Healthy establishment is fast (observed two-hop: well under a
    /// second); a bound of ~15s allows several WireGuard handshake
    /// retransmissions before declaring a hop dead.
    pub async fn await_established(
        &self,
        timeout: Duration,
    ) -> std::result::Result<(), NotEstablished> {
        let mut rx = self.established_rx.clone();
        let established = |(entry, exit): &(bool, Option<bool>)| *entry && exit.unwrap_or(true);
        // Drop the `watch::Ref` before touching `rx` again in the error path.
        let done = matches!(
            tokio::time::timeout(timeout, rx.wait_for(established)).await,
            Ok(Ok(_))
        );
        if done {
            Ok(())
        } else {
            // Timeout elapsed, or the datapath stopped (sender dropped): report
            // the last observed per-hop state either way.
            let (entry, exit) = *rx.borrow();
            Err(NotEstablished { entry, exit })
        }
    }

    /// The current per-hop MTU configuration.
    pub fn mtu(&self) -> MtuConfig {
        *self.mtu.read().expect("mtu lock poisoned")
    }

    /// Change the tunnel MTU at runtime. The WireGuard session is preserved (no
    /// re-handshake); the smol-core interface is rebuilt with the new MTU, so
    /// any sockets open at the moment of the change are reset. (A fully seamless
    /// in-place interface resize is not supported by `tokio-smoltcp`, which fixes
    /// the interface MTU at construction.)
    pub fn set_mtu(&self, mtu: MtuConfig) -> Result<()> {
        let interface_mtu = if self.two_hop { mtu.exit } else { mtu.entry };
        let (stack, channels) = build_stack(self.assigned, self.ipv6, self.dns, interface_mtu);
        // Serialise the whole swap so two concurrent calls can't interleave their channel-send and
        // stack-publish steps and leave `self.stack` and the datapath's channels out of sync.
        let _swap = self.swap_lock.lock().expect("swap lock poisoned");
        // Hand the running datapath the new stack's channels FIRST, then publish the new stack. The
        // datapath's `select!` is `biased` with the swap branch first, so it adopts the new channels
        // before it can observe the old `stack_out_rx` closing when the old stack is dropped here.
        self.swap_tx
            .unbounded_send(channels)
            .map_err(|_| DvpnError::Transport("datapath has stopped".into()))?;
        *self.stack.write().expect("stack lock poisoned") = Arc::new(stack);
        *self.mtu.write().expect("mtu lock poisoned") = mtu;
        info!(mtu = interface_mtu, "tunnel MTU updated at runtime");
        Ok(())
    }

    /// Subscribe to [`BandwidthEvent`]s from the monitor/top-up task. Works whether the tunnel was
    /// built with `bandwidth_topup` (auto top-up) or `bandwidth_monitor` (events only); with
    /// neither, no events are ever emitted.
    pub fn bandwidth_events(&self) -> broadcast::Receiver<BandwidthEvent> {
        self.events_tx.subscribe()
    }

    /// Open a TCP connection through the tunnel.
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        Ok(self.stack().tcp_connect(addr).await?)
    }

    /// Bind a UDP socket inside the tunnel (ephemeral port).
    pub async fn udp_socket(&self) -> Result<UdpSocket> {
        Ok(self.stack().udp_socket().await?)
    }

    /// Resolve a hostname through the tunnel.
    pub async fn resolve(&self, host: &str) -> Result<Vec<std::net::IpAddr>> {
        Ok(self.stack().resolve(host).await?)
    }

    /// Resolve `host` and open a TCP connection to it on `port`, through the tunnel.
    pub async fn tcp_connect_host(&self, host: &str, port: u16) -> Result<TcpStream> {
        Ok(self.stack().tcp_connect_host(host, port).await?)
    }

    /// A snapshot of the current smol-core stack (advanced use). A subsequent
    /// [`set_mtu`](Self::set_mtu) swaps the stack, so re-fetch after changing MTU.
    pub fn stack(&self) -> Arc<Stack> {
        self.stack.read().expect("stack lock poisoned").clone()
    }

    /// A `tower` connector that dials through this tunnel, for `tonic`/`hyper`.
    /// It tracks stack swaps, so it keeps working across a runtime MTU change.
    pub fn connector(&self) -> TunnelConnector {
        TunnelConnector::new(self.stack.clone())
    }

    /// Tear down the tunnel, stopping the datapath task. Issued tickets are
    /// retained (this crate never touches the credential store).
    ///
    /// Each background task is given a short grace period to observe the
    /// cancellation and exit; if it does not, it is aborted so `shutdown` always
    /// returns promptly.
    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        stop_task(self.task.take()).await;
        stop_task(self.topup_task.take()).await;
        info!("dVPN tunnel shut down");
    }
}

/// Grace period for a background task to exit after cancellation before it is
/// aborted.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

async fn stop_task(task: Option<JoinHandle<()>>) {
    let Some(mut handle) = task else { return };
    if tokio::time::timeout(SHUTDOWN_GRACE, &mut handle)
        .await
        .is_err()
    {
        handle.abort();
        let _ = handle.await;
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        // Fire-and-forget teardown if shutdown() was not called.
        self.cancel.cancel();
    }
}

fn as_socket_addr_v4(addr: SocketAddr, what: &str) -> Result<SocketAddrV4> {
    match addr {
        SocketAddr::V4(v4) => Ok(v4),
        SocketAddr::V6(_) => Err(DvpnError::Config(format!(
            "{what} must be IPv4 for two-hop inner framing"
        ))),
    }
}

/// The datapath task: the sole owner of the engine + transport halves.
async fn datapath(
    mut engine: WgEngine,
    mut sender: WgSender,
    mut receiver: WgReceiver,
    stack_channels: StackChannels,
    mut swap_rx: mpsc::UnboundedReceiver<StackChannels>,
    established_tx: watch::Sender<(bool, Option<bool>)>,
    cancel: CancellationToken,
) {
    let (mut stack_out_rx, mut stack_in_tx) = stack_channels;
    // Kick the initial handshake(s).
    let init = engine.initiate_handshakes();
    for pkt in init.to_network {
        if let Err(e) = sender.send(&pkt).await {
            warn!("failed to send handshake init: {e}");
        }
    }

    let mut ticker = tokio::time::interval(TIMER_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            // `biased`: check the swap and cancel branches before the data branches. This is what
            // makes a runtime MTU change safe: when `set_mtu` sends new channels and then drops the
            // old stack (closing `stack_out_rx`), both the swap branch and the `stack_out_rx`
            // closure become ready at once; taking the swap FIRST adopts the new channels so the
            // closed-`stack_out_rx` `None => break` never fires and the tunnel survives the resize.
            biased;

            // Runtime MTU change: swap to the rebuilt stack's channels while
            // keeping the WireGuard engine/session intact.
            maybe_swap = swap_rx.next() => {
                match maybe_swap {
                    Some((new_out_rx, new_in_tx)) => {
                        debug!("datapath swapping stack channels (runtime MTU change)");
                        stack_out_rx = new_out_rx;
                        stack_in_tx = new_in_tx;
                    }
                    // `swap_tx` was dropped — the tunnel is being torn down (Drop also fires
                    // `cancel`). Because this biased branch is polled first, we must break on its
                    // closure; otherwise `swap_rx.next()` returns `Ready(None)` every poll and
                    // starves the cancel branch, spinning the task at 100% CPU.
                    None => break,
                }
            }

            _ = cancel.cancelled() => {
                debug!("datapath cancelled");
                break;
            }

            maybe_app = stack_out_rx.next() => {
                match maybe_app {
                    Some(app) => {
                        let out = engine.encapsulate_app(&app);
                        send_all(&mut sender, out.to_network).await;
                    }
                    None => break, // stack gone
                }
            }

            incoming = receiver.recv() => {
                match incoming {
                    Ok(wg) => {
                        let out = engine.decapsulate_incoming(&wg);
                        // Publish establishment progress (no-op unless it changed).
                        let est = engine.establishment();
                        established_tx.send_if_modified(|v| {
                            let changed = *v != est;
                            if changed { *v = est; }
                            changed
                        });
                        for pkt in out.to_stack {
                            if stack_in_tx.unbounded_send(pkt).is_err() {
                                debug!("stack channel closed");
                                return;
                            }
                        }
                        send_all(&mut sender, out.to_network).await;
                    }
                    Err(e) => {
                        if receiver.is_bridge() {
                            // A closed QUIC bridge stream cannot recover: stop the datapath rather
                            // than spin re-reading a permanently failed transport.
                            warn!("bridge transport failed, stopping datapath: {e}");
                            break;
                        }
                        // Direct UDP: transient. Back off briefly so a persistently failing socket
                        // doesn't busy-loop the CPU.
                        debug!("transport recv error (transient): {e}");
                        tokio::time::sleep(TRANSPORT_ERROR_BACKOFF).await;
                    }
                }
            }

            _ = ticker.tick() => {
                let out = engine.update_timers();
                send_all(&mut sender, out.to_network).await;
            }
        }
    }
}

async fn send_all(sender: &mut WgSender, packets: Vec<Vec<u8>>) {
    for pkt in packets {
        if let Err(e) = sender.send(&pkt).await {
            warn!("transport send error: {e}");
        }
    }
}
