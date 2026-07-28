// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # smoldvpn
//!
//! A pure-Rust, userspace 1-/2-hop WireGuard dVPN datapath built on
//! [`boringtun`](https://docs.rs/boringtun) and [`nym_smol_core`], with **no OS
//! `tun` device and no root**. Application traffic flows through the tunnel via
//! ordinary tokio socket surfaces ([`TcpStream`], [`UdpSocket`], and the
//! `tonic`/`hyper`/`reqwest` connectors in [`connectors`]).
//!
//! The datapath is decoupled from provisioning: build a [`PeerConfig`] per hop
//! (e.g. from a `nym-sdk-session` registration) and hand it to a
//! [`TunnelBuilder`]. Three data-plane modes are supported: one-hop, two-hop,
//! and QUIC-tunnelling two-hop (see [`BridgeParams`]).
//!
//! ```no_run
//! # async fn example(entry: nym_smoldvpn::PeerConfig, exit: nym_smoldvpn::PeerConfig)
//! # -> Result<(), Box<dyn std::error::Error>> {
//! use nym_smoldvpn::TunnelBuilder;
//!
//! let tunnel = TunnelBuilder::two_hop(entry, exit).connect().await?;
//! let mut tcp = tunnel.tcp_connect("1.1.1.1:443".parse()?).await?;
//! // ... use `tcp` as any AsyncRead + AsyncWrite ...
//! tunnel.shutdown().await;
//! # Ok(())
//! # }
//! ```

mod bridge;
mod config;
mod connectors;
mod engine;
mod error;
mod framing;
mod topup;
mod transport;
mod tunnel;

pub use bridge::{probe as probe_bridge, BridgeParams};
pub use config::{DnsMode, MtuConfig, PeerConfig, TunnelConfig, DEFAULT_EXIT_WG_CLIENT_PORT};
pub use connectors::TunnelConnector;
pub use error::{DvpnError, Result};
pub use topup::{
    query_available_bandwidth, topup_bandwidth, BandwidthCredentialSource, BandwidthEvent,
    CredentialFuture, ProviderCredentialSource, TopupConfig,
};
pub use transport::SocketProtector;
pub use tunnel::{NotEstablished, Tunnel, TunnelBuilder};

/// tokio socket types produced by the tunnel (re-exported from `smol-core`).
pub use nym_smol_core::{TcpStream, UdpSocket};
