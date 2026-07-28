// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # smol-core
//!
//! A transport-agnostic, pure-Rust userspace TCP/IP stack. It turns a
//! bidirectional stream of raw IP packets (`Vec<u8>`) into tokio-native
//! [`TcpStream`] / [`UdpSocket`] sockets plus a tunnel-scoped DNS resolver,
//! with **no OS `tun` device and no elevated privileges**. It does not depend
//! on Go, a gVisor netstack, or any FFI network stack.
//!
//! The core abstraction is the IP-packet transport: anything that can produce
//! inbound IP packets and consume outbound ones drives the same stack. Provide
//! a [`ChannelDevice`] fed from your transport's channels (a mixnet bridge, a
//! WireGuard datapath, a test harness), build a [`Stack`], and open sockets:
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use futures::channel::mpsc;
//! use nym_smol_core::{ChannelDevice, Stack, StackConfig, DEFAULT_MTU};
//!
//! // `inbound_*` carry IP packets from the transport into the stack;
//! // `outbound_*` carry stack-produced IP packets back to the transport.
//! let (outbound_tx, _outbound_rx) = mpsc::unbounded::<Vec<u8>>();
//! let (_inbound_tx, inbound_rx) = mpsc::unbounded::<Vec<u8>>();
//!
//! let device = ChannelDevice::new(inbound_rx, outbound_tx, Some(DEFAULT_MTU));
//! let stack = Stack::new(device, StackConfig::new("10.0.0.2".parse()?));
//!
//! let _tcp = stack.tcp_connect("1.1.1.1:443".parse()?).await?;
//! # Ok(())
//! # }
//! ```
//!
//! `smolmix` (the 5-hop mixnet tunnel) is built on top of this crate.

mod device;
mod dns;
mod error;
mod stack;

pub use device::{ChannelDevice, DEFAULT_MTU};
pub use dns::{DnsConfig, DEFAULT_DNS_SERVER, DEFAULT_QUERY_TIMEOUT};
pub use error::{Result, SmolCoreError};
pub use stack::{Stack, StackConfig, TcpStream, UdpSocket};
