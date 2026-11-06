// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The data plane's half of talking to a gateway: already-encrypted packets out, whatever arrives
//! in.

use crate::error::Result;
use nym_lp::transport::traits::LpDatagramChannel;
use nym_lp_data::packet::EncryptedLpPacket;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

/// One socket for every gateway the client talks to.
///
/// Separate from [`LpGatewayControlClient`](crate::LpGatewayControlClient) because the two planes
/// want opposite things from ownership. A control connection belongs to one caller at a time and is
/// mutated by the handshake running over it; the data socket is shared by everything that sends and
/// by whoever runs the receive loop, and none of them mutate it. So this is `Clone` and takes
/// `&self` throughout, and the control client is neither.
///
/// One socket rather than one per gateway: a gateway answers to whatever address a packet came
/// from, so nothing has to be known in advance and nothing has to be rebound when the set of
/// gateways changes.
///
/// Generic over the channel so a test can swap in an in-memory pair; `UdpSocket` in production.
#[derive(Clone)]
pub struct LpGatewayDataClient<D = UdpSocket> {
    socket: Arc<D>,
}

impl<D> LpGatewayDataClient<D>
where
    D: LpDatagramChannel,
{
    /// Take a local address. `[::]:0` takes an ephemeral port on every interface.
    pub async fn bind(local: SocketAddr) -> Result<Self> {
        Ok(LpGatewayDataClient {
            socket: Arc::new(D::bind(local).await?),
        })
    }

    /// The address this ended up on, once the OS has chosen a port.
    pub fn local_address(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_address()?)
    }

    /// Send this there.
    ///
    /// The packet is already encrypted; this neither knows nor cares which session made it, which
    /// is why the destination has to be named.
    pub async fn send(&self, packet: &EncryptedLpPacket, dst: SocketAddr) -> Result<()> {
        Ok(self.socket.send_packet_to(packet, dst).await?)
    }

    /// The next packet off the socket, and who sent it.
    pub async fn recv(&self) -> Result<(EncryptedLpPacket, SocketAddr)> {
        Ok(self.socket.receive_packet_from().await?)
    }
}
