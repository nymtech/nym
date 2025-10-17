// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::mixnet_stream_wrapper::{MixStream, MixStreamReader, MixStreamWriter};
use crate::ip_packet_client::{
    helpers::check_ipr_message_version, IprListener, MixnetMessageOutcome,
};
use crate::UserAgent;
use crate::{mixnet::Recipient, Error};

use bytes::Bytes;
use nym_gateway_directory::{
    Config as GatewayConfig, GatewayClient, GatewayType, IpPacketRouterAddress,
};
use nym_ip_packet_requests::{
    v8::{
        request::IpPacketRequest,
        response::{ConnectResponseReply, ControlResponse, IpPacketResponse, IpPacketResponseData},
    },
    IpPair,
};
use nym_sphinx::receiver::ReconstructedMessageCodec;

use futures::StreamExt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::oneshot;
use tokio_util::codec::FramedRead;
use tracing::{debug, error, info};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

fn create_gateway_client() -> Result<GatewayClient, Error> {
    // TODO do something proper with this
    let user_agent = UserAgent {
        application: "nym-ipr-streamer".to_string(),
        version: "0.0.1".to_string(),
        platform: "rust".to_string(),
        git_commit: "max/sdk-streamer".to_string(),
    };

    let mainnet_network_defaults = crate::NymNetworkDetails::default();
    let api_url = mainnet_network_defaults
        .endpoints
        .first()
        .ok_or_else(|| Error::NoValidatorDetailsAvailable)?
        .api_url()
        .ok_or_else(|| Error::NoValidatorAPIUrl)?;

    let nyxd_url = mainnet_network_defaults
        .endpoints
        .first()
        .ok_or_else(|| Error::NoValidatorDetailsAvailable)?
        .nyxd_url();

    let nym_vpn_api_url = mainnet_network_defaults
        .nym_vpn_api_url()
        .ok_or_else(|| Error::NoNymAPIUrl)?;

    let config = GatewayConfig {
        nyxd_url,
        api_url,
        nym_vpn_api_url: Some(nym_vpn_api_url),
        min_gateway_performance: None,
        mix_score_thresholds: None,
        wg_score_thresholds: None,
    };

    Ok(GatewayClient::new(config, user_agent)?)
}

async fn get_ipr_addr(client: GatewayClient) -> Result<IpPacketRouterAddress, Error> {
    let exit_gateways = client.lookup_gateways(GatewayType::MixnetExit).await?;

    info!("Found {} Exit Gateways", exit_gateways.len());

    let selected_gateway = exit_gateways
        .into_iter()
        .filter(|gw| gw.ipr_address.is_some())
        .max_by_key(|gw| {
            gw.mixnet_performance
                .map(|p| p.round_to_integer())
                .unwrap_or(0)
        })
        .ok_or_else(|| Error::NoGatewayAvailable)?;

    let ipr_address = selected_gateway
        .ipr_address
        .ok_or_else(|| Error::NoIPRAvailable)?;

    info!(
        "Using IPR: {} (Gateway: {}, Performance: {:?})",
        ipr_address,
        selected_gateway.identity(),
        selected_gateway.mixnet_performance
    );

    Ok(ipr_address)
}

/// Unlike the non-IPR MixStream, we do not start with a Socket and then 'connect' to a Stream; seemed too many layers of abstraction for little trade off.
pub struct IpMixStream {
    stream: MixStream,
    ipr_address: IpPacketRouterAddress,
    listener: IprListener,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
}

impl IpMixStream {
    // TODO be able to pass in DisconnectedMixnetClient to use as MixStream inner client.
    pub async fn new() -> Result<Self, Error> {
        let gw_client = create_gateway_client()?;
        let ipr_address = get_ipr_addr(gw_client).await?;
        let stream = MixStream::new(None, Recipient::from(ipr_address)).await;

        Ok(Self {
            stream,
            ipr_address,
            listener: IprListener::new(),
            allocated_ips: None,
            connection_state: ConnectionState::Disconnected,
        })
    }

    pub fn nym_address(&self) -> &Recipient {
        self.stream.client.nym_address()
    }

    async fn send_ipr_request(&mut self, request: IpPacketRequest) -> Result<(), Error> {
        let request_bytes = request.to_bytes()?;
        self.stream.write_bytes(&request_bytes).await
    }

    pub async fn connect_tunnel(&mut self) -> Result<IpPair, Error> {
        if self.connection_state != ConnectionState::Disconnected {
            return Err(Error::IprStreamClientAlreadyConnectedOrConnecting);
        }

        self.connection_state = ConnectionState::Connecting;
        info!("Connecting to IP packet router: {}", self.ipr_address);

        match self.connect_inner().await {
            Ok(ip_pair) => {
                self.allocated_ips = Some(ip_pair.clone());
                self.connection_state = ConnectionState::Connected;
                info!(
                    "Connected to IPv4: {}, IPv6: {}",
                    ip_pair.ipv4, ip_pair.ipv6
                );
                Ok(ip_pair)
            }
            Err(e) => {
                self.connection_state = ConnectionState::Disconnected;
                error!("Failed to connect: {:?}", e);
                Err(e)
            }
        }
    }

    async fn connect_inner(&mut self) -> Result<IpPair, Error> {
        let (request, request_id) = IpPacketRequest::new_connect_request(None);
        debug!("Sending connect request with ID: {}", request_id);

        self.send_ipr_request(request).await?;
        self.listen_for_connect_response(request_id).await
    }

    async fn listen_for_connect_response(&mut self, request_id: u64) -> Result<IpPair, Error> {
        let timeout = tokio::time::sleep(IPR_CONNECT_TIMEOUT);
        tokio::pin!(timeout);

        let mut framed = FramedRead::new(&mut self.stream, ReconstructedMessageCodec {});

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::IPRConnectResponseTimeout);
                }
                frame = framed.next() => {
                    match frame {
                        None => {
                            return Err(Error::IPRClientStreamClosed);
                        }
                        Some(Err(e)) => {
                            return Err(Error::MessageRecovery(e));
                        }
                        Some(Ok(reconstructed)) => {
                            if let Err(e) = check_ipr_message_version(&reconstructed) {
                                error!("Version check failed: {}", e);
                                continue;
                            }
                            if let Ok(response) = IpPacketResponse::from_reconstructed_message(&reconstructed) {
                                if response.id() == Some(request_id) {
                                    return self.handle_connect_response(response).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_connect_response(&self, response: IpPacketResponse) -> Result<IpPair, Error> {
        let control_response = match response.data {
            IpPacketResponseData::Control(c) => c,
            other => return Err(Error::UnexpectedResponseType(other)),
        };

        match *control_response {
            ControlResponse::Connect(connect_resp) => match connect_resp.reply {
                ConnectResponseReply::Success(success) => Ok(success.ips),
                ConnectResponseReply::Failure(reason) => Err(Error::ConnectDenied(reason)),
            },
            _ => Err(Error::UnexpectedResponseType(
                IpPacketResponseData::Control(control_response.clone()),
            )),
        }
    }

    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        if self.connection_state != ConnectionState::Connected {
            return Err(Error::IprStreamClientNotConnected);
        }
        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        self.send_ipr_request(request).await
    }

    pub async fn handle_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        let mut framed = FramedRead::new(&mut self.stream, ReconstructedMessageCodec {});

        match tokio::time::timeout(Duration::from_secs(10), framed.next()).await {
            Ok(Some(reconstructed)) => {
                match self
                    .listener
                    .handle_reconstructed_message(reconstructed?)
                    .await
                {
                    Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                        info!("Extracted {} IP packets", packets.len());
                        Ok(packets)
                    }
                    Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                        info!("Received disconnect");
                        self.connection_state = ConnectionState::Disconnected;
                        self.allocated_ips = None;
                        Ok(Vec::new())
                    }
                    Ok(Some(MixnetMessageOutcome::MixnetSelfPing)) => {
                        debug!("Received mixnet self ping");
                        Ok(Vec::new())
                    }
                    Ok(None) => Ok(Vec::new()),
                    Err(e) => {
                        error!("Failed to handle message: {}", e);
                        Ok(Vec::new())
                    }
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Disconnect inner stream client from the Mixnet - note that disconnected clients cannot currently be reconnected.
    pub async fn disconnect_stream(self) {
        debug!("Disconnecting");
        self.stream.disconnect().await;
        debug!("Disconnected");
    }

    /// Split for concurrent read/write (like TcpStream::Split) into IpMixnetStreamReader and IpMixnetStreamWriter.
    pub fn split(self) -> (IpMixStreamReader, IpMixStreamWriter) {
        debug!("Splitting IpMixStream");
        let local_addr = self.stream.client.nym_address().clone();
        let (stream_reader, stream_writer) = self.stream.split();
        debug!("Split IpMixStream into Reader and Writer");

        let (state_tx, state_rx) = oneshot::channel();
        let (ips_tx, ips_rx) = oneshot::channel();

        (
            IpMixStreamReader {
                stream_reader,
                // ipr_address: self.ipr_address,
                listener: self.listener,
                allocated_ips: self.allocated_ips.clone(),
                connection_state: self.connection_state.clone(),
                state_tx: Some(state_tx),
                ips_tx: Some(ips_tx),
            },
            IpMixStreamWriter {
                stream_writer,
                // ipr_address: self.ipr_address,
                local_addr,
                allocated_ips: self.allocated_ips,
                connection_state: self.connection_state,
                state_rx: Some(state_rx),
                ips_rx: Some(ips_rx),
            },
        )
    }
}

impl AsyncRead for IpMixStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for IpMixStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

pub struct IpMixStreamReader {
    stream_reader: MixStreamReader,
    // ipr_address: IpPacketRouterAddress,
    listener: IprListener,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
    state_tx: Option<oneshot::Sender<ConnectionState>>,
    ips_tx: Option<oneshot::Sender<Option<IpPair>>>,
}

impl IpMixStreamReader {
    pub fn nym_address(&self) -> &Recipient {
        self.stream_reader.local_addr()
    }

    pub async fn handle_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        let mut framed = FramedRead::new(&mut self.stream_reader, ReconstructedMessageCodec {});

        match tokio::time::timeout(Duration::from_secs(10), framed.next()).await {
            Ok(Some(reconstructed)) => {
                match self
                    .listener
                    .handle_reconstructed_message(reconstructed?)
                    .await
                {
                    Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                        info!("Extracted {} IP packets", packets.len());
                        Ok(packets)
                    }
                    Ok(Some(MixnetMessageOutcome::Disconnect)) => {
                        info!("Received disconnect");
                        self.connection_state = ConnectionState::Disconnected;
                        self.allocated_ips = None;
                        // Send state update to writer
                        if let Some(tx) = self.state_tx.take() {
                            let _ = tx.send(ConnectionState::Disconnected);
                        }
                        if let Some(tx) = self.ips_tx.take() {
                            let _ = tx.send(None);
                        }
                        Ok(Vec::new())
                    }
                    Ok(Some(MixnetMessageOutcome::MixnetSelfPing)) => {
                        debug!("Received mixnet self ping");
                        Ok(Vec::new())
                    }
                    Ok(None) => Ok(Vec::new()),
                    Err(e) => {
                        error!("Failed to handle message: {}", e);
                        Ok(Vec::new())
                    }
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
}

impl AsyncRead for IpMixStreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_reader).poll_read(cx, buf)
    }
}

pub struct IpMixStreamWriter {
    stream_writer: MixStreamWriter,
    // ipr_address: IpPacketRouterAddress,
    local_addr: Recipient,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
    state_rx: Option<oneshot::Receiver<ConnectionState>>,
    ips_rx: Option<oneshot::Receiver<Option<IpPair>>>,
}

impl IpMixStreamWriter {
    pub fn nym_address(&self) -> &Recipient {
        &self.local_addr
    }

    async fn send_ipr_request(&mut self, request: IpPacketRequest) -> Result<(), Error> {
        let request_bytes = request.to_bytes()?;
        self.stream_writer.write_bytes(&request_bytes).await
    }

    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        // Check for state updates from reader
        if let Some(mut rx) = self.state_rx.take() {
            if let Ok(new_state) = rx.try_recv() {
                self.connection_state = new_state;
            } else {
                self.state_rx = Some(rx);
            }
        }

        if let Some(mut rx) = self.ips_rx.take() {
            if let Ok(new_ips) = rx.try_recv() {
                self.allocated_ips = new_ips;
            } else {
                self.ips_rx = Some(rx);
            }
        }

        if self.connection_state != ConnectionState::Connected {
            return Err(Error::IprStreamClientNotConnected);
        }

        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        self.send_ipr_request(request).await
    }

    pub fn allocated_ips(&self) -> Option<&IpPair> {
        self.allocated_ips.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }
}

impl AsyncWrite for IpMixStreamWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream_writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream_writer).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_packet_client::helpers::{
        icmp_identifier, is_icmp_echo_reply, is_icmp_v6_echo_reply, send_ping_v4, send_ping_v6,
    };
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_logging() {
        if tracing::dispatcher::has_been_set() {
            return;
        }
        INIT.call_once(|| {
            nym_bin_common::logging::setup_tracing_logger();
        });
    }

    #[tokio::test]
    async fn connect_to_ipr() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let mut stream = IpMixStream::new().await?;
        let ip_pair = stream.connect_tunnel().await?;

        let ipv4: Ipv4Addr = ip_pair.ipv4;
        assert!(!ipv4.is_unspecified(), "IPv4 address should not be 0.0.0.0");

        let ipv6: Ipv6Addr = ip_pair.ipv6;
        assert!(!ipv6.is_unspecified(), "IPv6 address should not be ::");

        assert!(stream.is_connected(), "Stream should be connected");
        assert!(
            stream.allocated_ips().is_some(),
            "Should have allocated IPs"
        );

        stream.disconnect_stream().await;

        Ok(())
    }

    #[tokio::test]
    async fn dns_ping_checks() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let mut stream = IpMixStream::new().await?;
        let ip_pair = stream.connect_tunnel().await?;

        info!(
            "Connected with IPs - IPv4: {}, IPv6: {}",
            ip_pair.ipv4, ip_pair.ipv6
        );

        let external_v4_targets = vec![
            ("Google DNS", Ipv4Addr::new(8, 8, 8, 8)),
            ("Cloudflare DNS", Ipv4Addr::new(1, 1, 1, 1)),
            ("Quad9 DNS", Ipv4Addr::new(9, 9, 9, 9)),
        ];

        let external_v6_targets = vec![
            ("Google DNS", "2001:4860:4860::8888".parse::<Ipv6Addr>()?),
            (
                "Cloudflare DNS",
                "2606:4700:4700::1111".parse::<Ipv6Addr>()?,
            ),
            ("Quad9 DNS", "2620:fe::fe".parse::<Ipv6Addr>()?),
        ];

        let identifier = icmp_identifier();
        let mut successful_v4_pings = 0;
        let mut total_v4_pings = 0;
        let mut successful_v6_pings = 0;
        let mut total_v6_pings = 0;

        for (name, target) in &external_v4_targets {
            info!("Testing IPv4 connectivity to {} ({})", name, target);

            for seq in 0..3 {
                send_ping_v4(&mut stream, &ip_pair, seq, identifier, *target).await?;
                total_v4_pings += 1;
            }
        }

        for (name, target) in &external_v6_targets {
            info!("Testing IPv6 connectivity to {} ({})", name, target);

            for seq in 0..3 {
                send_ping_v6(&mut stream, &ip_pair, seq, *target).await?;
                total_v6_pings += 1;
            }
        }

        let collect_timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(collect_timeout);

        loop {
            tokio::select! {
                _ = &mut collect_timeout => {
                    info!("Finished collecting replies");
                    break;
                }
                result = stream.handle_incoming() => {
                    if let Ok(packets) = result {
                        for packet in packets {
                            if let Some((reply_id, source, dest)) = is_icmp_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv4 {
                                    successful_v4_pings += 1;
                                    debug!("IPv4 reply from {}", source);
                                }
                            }

                            if let Some((reply_id, source, dest)) = is_icmp_v6_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv6 {
                                    successful_v6_pings += 1;
                                    debug!("IPv6 reply from {}", source);
                                }
                            }
                        }
                    }
                }
            }
        }

        let v4_success_rate = (successful_v4_pings as f64 / total_v4_pings as f64) * 100.0;
        let v6_success_rate = (successful_v6_pings as f64 / total_v6_pings as f64) * 100.0;

        info!(
            "IPv4 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v4_pings, total_v4_pings, v4_success_rate
        );
        info!(
            "IPv6 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v6_pings, total_v6_pings, v6_success_rate
        );

        assert!(successful_v4_pings > 0, "No IPv4 pings successful");
        assert!(
            v4_success_rate >= 75.0,
            "IPv4 success rate < 75% (got {:.1}%)",
            v4_success_rate
        );

        assert!(successful_v6_pings > 0, "No IPv6 pings successful");
        assert!(
            v6_success_rate >= 75.0,
            "IPv6 success rate < 75% (got {:.1}%)",
            v6_success_rate
        );

        stream.disconnect_stream().await;
        Ok(())
    }

    #[tokio::test]
    async fn split_dns_ping_checks() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let mut stream = IpMixStream::new().await?;
        let ip_pair = stream.connect_tunnel().await?;

        info!(
            "Connected with IPs - IPv4: {}, IPv6: {}",
            ip_pair.ipv4, ip_pair.ipv6
        );

        let (mut reader, mut writer) = stream.split();

        let external_v4_targets = vec![("Google DNS", Ipv4Addr::new(8, 8, 8, 8))];

        let identifier = icmp_identifier();
        let mut successful_v4_pings = 0;
        let mut total_v4_pings = 0;

        for (name, target) in &external_v4_targets {
            info!("Testing IPv4 connectivity to {} ({})", name, target);

            for seq in 0..2 {
                use crate::ip_packet_client::helpers::{
                    create_icmpv4_echo_request, wrap_icmp_in_ipv4,
                };
                use nym_ip_packet_requests::codec::MultiIpPacketCodec;
                use pnet_packet::Packet;

                let icmp_echo_request = create_icmpv4_echo_request(seq, identifier)?;
                let ipv4_packet = wrap_icmp_in_ipv4(icmp_echo_request, ip_pair.ipv4, *target)?;
                let bundled_packet =
                    MultiIpPacketCodec::bundle_one_packet(ipv4_packet.packet().to_vec().into());

                writer.send_ip_packet(&bundled_packet).await?;
                total_v4_pings += 1;
            }
        }

        let collect_timeout = tokio::time::sleep(Duration::from_secs(10));
        tokio::pin!(collect_timeout);

        loop {
            tokio::select! {
                _ = &mut collect_timeout => {
                    info!("Finished collecting responses");
                    break;
                }
                result = reader.handle_incoming() => {
                    if let Ok(packets) = result {
                        for packet in packets {
                            if let Some((reply_id, source, dest)) = is_icmp_echo_reply(&packet) {
                                if reply_id == identifier && dest == ip_pair.ipv4 {
                                    successful_v4_pings += 1;
                                    debug!("IPv4 reply from {}", source);
                                }
                            }
                        }
                    }
                }
            }
        }

        let v4_success_rate = if total_v4_pings > 0 {
            (successful_v4_pings as f64 / total_v4_pings as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "Split test - IPv4 external connectivity: {}/{} pings successful ({:.1}%)",
            successful_v4_pings, total_v4_pings, v4_success_rate
        );

        assert!(successful_v4_pings > 0, "No pings successful");
        assert!(
            v4_success_rate >= 75.0,
            "IPv4 success rate < 75% (got {:.1}%)",
            v4_success_rate
        );

        Ok(())
    }
}
