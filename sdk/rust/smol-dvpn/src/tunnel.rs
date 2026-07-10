// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! The tunnel: builder, lifecycle, and the datapath task.
//!
//! A [`Tunnel`] owns a smol-core [`Stack`] (the tokio socket surface) and spawns
//! a single datapath task that shuttles packets between the stack and the active
//! WireGuard transport, encrypting/decrypting via the [`WgEngine`]. The task is
//! the sole owner of the `Tunn`s (no locking) and is driven by a `select!` over:
//! outbound app packets, inbound WireGuard packets, a boringtun timer tick, and
//! the `CancellationToken`.

use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc;
use futures::StreamExt;
use smol_core::{ChannelDevice, DnsConfig, Stack, StackConfig, TcpStream, UdpSocket};

use crate::connectors::TunnelConnector;
use crate::topup::{run_topup, BandwidthCredentialSource, TopupConfig};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::bridge::{self, BridgeParams};
use crate::config::{DnsMode, PeerConfig, TunnelConfig};
use crate::engine::WgEngine;
use crate::error::{DvpnError, Result};
use crate::transport::{direct_transport, SocketProtector, WgReceiver, WgSender};

/// boringtun timer pump interval.
const TIMER_INTERVAL: Duration = Duration::from_millis(250);

/// Which data-plane transport to use.
#[derive(Clone, Debug)]
enum TransportChoice {
    /// Real UDP datagrams to the entry gateway.
    Direct,
    /// QUIC bridge fronting the two-hop entry leg.
    Quic(BridgeParams),
}

/// Builder for a [`Tunnel`].
pub struct TunnelBuilder {
    entry: PeerConfig,
    exit: Option<PeerConfig>,
    config: TunnelConfig,
    transport: TransportChoice,
    protector: Option<SocketProtector>,
    cancel: Option<CancellationToken>,
    topup: Option<(TopupConfig, Arc<dyn BandwidthCredentialSource>)>,
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
    /// endpoint and spend stored tickets (via `source`) before the registered
    /// bandwidth is exhausted. Stops with the tunnel.
    pub fn bandwidth_topup(
        mut self,
        config: TopupConfig,
        source: Arc<dyn BandwidthCredentialSource>,
    ) -> Self {
        self.topup = Some((config, source));
        self
    }

    /// Bring up the tunnel.
    pub async fn connect(self) -> Result<Tunnel> {
        Tunnel::connect(self).await
    }
}

/// A running dVPN tunnel exposing tokio socket surfaces.
pub struct Tunnel {
    stack: Arc<Stack>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
    topup_task: Option<JoinHandle<()>>,
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

        // Channels between the stack and the datapath task.
        //   stack_out: stack -> datapath (app IP packets to encrypt)
        //   stack_in:  datapath -> stack (decrypted IP packets)
        let (stack_out_tx, stack_out_rx) = mpsc::unbounded::<Vec<u8>>();
        let (stack_in_tx, stack_in_rx) = mpsc::unbounded::<Vec<u8>>();

        let device = ChannelDevice::new(stack_in_rx, stack_out_tx, interface_mtu);
        let mut stack_config = StackConfig::new(assigned).with_mtu(interface_mtu);
        if let Some(v6) = ipv6 {
            stack_config = stack_config.with_ipv6(v6);
        }
        let mut stack = Stack::new(device, stack_config);
        if let DnsMode::InTunnelServer(server) = builder.config.dns {
            stack = stack.with_dns_config(DnsConfig {
                server,
                ..DnsConfig::default()
            });
        }

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
            "dVPN tunnel datapath starting"
        );

        let task = tokio::spawn(datapath(
            engine,
            sender,
            receiver,
            stack_out_rx,
            stack_in_tx,
            cancel.clone(),
        ));

        // Optional background bandwidth top-up task.
        let topup_task = builder
            .topup
            .map(|(cfg, source)| tokio::spawn(run_topup(cfg, source, cancel.clone())));

        Ok(Tunnel {
            stack: Arc::new(stack),
            cancel,
            task: Some(task),
            topup_task,
        })
    }

    /// Open a TCP connection through the tunnel.
    pub async fn tcp_connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        Ok(self.stack.tcp_connect(addr).await?)
    }

    /// Bind a UDP socket inside the tunnel (ephemeral port).
    pub async fn udp_socket(&self) -> Result<UdpSocket> {
        Ok(self.stack.udp_socket().await?)
    }

    /// Resolve a hostname through the tunnel.
    pub async fn resolve(&self, host: &str) -> Result<Vec<std::net::IpAddr>> {
        Ok(self.stack.resolve(host).await?)
    }

    /// Resolve `host` and open a TCP connection to it on `port`, through the tunnel.
    pub async fn tcp_connect_host(&self, host: &str, port: u16) -> Result<TcpStream> {
        Ok(self.stack.tcp_connect_host(host, port).await?)
    }

    /// Access the underlying smol-core stack (advanced use).
    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    /// A `tower` connector that dials through this tunnel, for `tonic`/`hyper`.
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
    mut stack_out_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    stack_in_tx: mpsc::UnboundedSender<Vec<u8>>,
    cancel: CancellationToken,
) {
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
                        for pkt in out.to_stack {
                            if stack_in_tx.unbounded_send(pkt).is_err() {
                                debug!("stack channel closed");
                                return;
                            }
                        }
                        send_all(&mut sender, out.to_network).await;
                    }
                    Err(e) => {
                        // Transient on Direct UDP; fatal on a closed bridge stream.
                        debug!("transport recv error: {e}");
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
