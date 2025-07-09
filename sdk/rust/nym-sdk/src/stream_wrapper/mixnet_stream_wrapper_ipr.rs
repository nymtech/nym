use crate::UserAgent;
use crate::{mixnet::Recipient, Error};

use super::mixnet_stream_wrapper::{MixSocket, MixStream};

use bytes::Bytes;
use bytes::BytesMut;
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
use std::sync::Once;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{Decoder, FramedRead};
use tracing::{debug, error, info};

use crate::ip_packet_client::{
    helpers::check_ipr_message_version,
    listener::{IprListener, MixnetMessageOutcome},
};

const IPR_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

pub struct IpMixSocket {
    inner: MixSocket,
    gateway_client: GatewayClient,
}

impl IpMixSocket {
    pub async fn new() -> Result<Self, Error> {
        let inner = MixSocket::new().await?;
        let gateway_client = Self::create_gateway_client()?;
        Ok(Self {
            inner,
            gateway_client,
        })
    }

    fn create_gateway_client() -> Result<GatewayClient, Error> {
        let user_agent = UserAgent {
            application: "nym-ipr-streamer".to_string(),
            version: "0.0.1".to_string(),
            platform: "xxxxxxx".to_string(),
            git_commit: "".to_string(),
        };

        let mainnet_network_defaults = crate::NymNetworkDetails::default();
        let api_url = mainnet_network_defaults
            .endpoints
            .first()
            .unwrap()
            .api_url()
            .unwrap();

        let nyxd_url = mainnet_network_defaults
            .endpoints
            .first()
            .unwrap()
            .nyxd_url();

        let nym_vpn_api_url = mainnet_network_defaults.nym_vpn_api_url().unwrap();

        let config = GatewayConfig {
            nyxd_url,
            api_url,
            nym_vpn_api_url: Some(nym_vpn_api_url),
            min_gateway_performance: None,
            mix_score_thresholds: None,
            wg_score_thresholds: None,
        };

        Ok(GatewayClient::new(config, user_agent).unwrap())
    }

    async fn get_best_ipr_address(&self) -> Result<IpPacketRouterAddress, Error> {
        let exit_gateways = self
            .gateway_client
            .lookup_gateways(GatewayType::MixnetExit)
            .await
            .unwrap();

        info!("Found {} Exit Gateways", exit_gateways.len());

        let selected_gateway = exit_gateways
            .into_iter()
            .filter(|gw| gw.has_ipr_address())
            .max_by_key(|gw| {
                gw.mixnet_performance
                    .map(|p| p.round_to_integer())
                    .unwrap_or(0)
            })
            .unwrap();

        let ipr_address = selected_gateway.ipr_address.unwrap();

        info!(
            "Using IPR: {} (Gateway: {}, Performance: {:?})",
            ipr_address,
            selected_gateway.identity(),
            selected_gateway.mixnet_performance
        );

        Ok(ipr_address)
    }

    pub async fn connect(&self) -> Result<IpMixStream, Error> {
        let ipr_address = self.get_best_ipr_address().await?;
        let stream = MixStream::new(None, Recipient::from(ipr_address.clone())).await;
        Ok(IpMixStream::new(stream, ipr_address))
    }

    pub fn nym_address(&self) -> &Recipient {
        self.inner.nym_address()
    }
}

pub struct IpMixStream {
    stream: MixStream,
    ipr_address: IpPacketRouterAddress,
    listener: IprListener,
    allocated_ips: Option<IpPair>,
    connection_state: ConnectionState,
}

impl IpMixStream {
    fn new(stream: MixStream, ipr_address: IpPacketRouterAddress) -> Self {
        Self {
            stream,
            ipr_address,
            listener: IprListener::new(),
            allocated_ips: None,
            connection_state: ConnectionState::Disconnected,
        }
    }

    async fn send_ipr_request(&mut self, request: IpPacketRequest) -> Result<(), Error> {
        let request_bytes = request.to_bytes()?;
        self.stream.write_bytes(&request_bytes).await
    }

    pub async fn connect_tunnel(&mut self) -> Result<IpPair, Error> {
        if self.connection_state != ConnectionState::Disconnected {
            return Err(Error::new_unsupported(
                "Already connected or connecting".to_string(),
            ));
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

        let mut buffer = vec![0u8; 65536];

        loop {
            tokio::select! {
                _ = &mut timeout => {
                    return Err(Error::new_unsupported("Timeout waiting for connect response".to_string()));
                }
                result = self.stream.read(&mut buffer) => {
                    match result {
                        Ok(0) => {
                            debug!("Stream closed");
                            return Err(Error::new_unsupported("Stream closed".to_string()));
                        }
                        Ok(n) => {
                            debug!("Received {} bytes", n);

                            let mut codec = ReconstructedMessageCodec {};
                            let mut buf = BytesMut::from(&buffer[..n]);

                            if let Ok(Some(reconstructed)) = codec.decode(&mut buf) {
                                if let Err(e) = check_ipr_message_version(&reconstructed) {
                                    debug!("Version check failed: {}", e);
                                    continue;
                                }
                                if let Ok(response) = IpPacketResponse::from_reconstructed_message(&reconstructed) {
                                    if response.id() == Some(request_id) {
                                        return self.handle_connect_response(response).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Read error: {}", e);
                            return Err(Error::new_unsupported(format!("Read error: {}", e)));
                        }
                    }
                }
            }
        }
    }

    async fn handle_connect_response(&self, response: IpPacketResponse) -> Result<IpPair, Error> {
        let control_response = match response.data {
            IpPacketResponseData::Control(control) => control,
            _ => {
                return Err(Error::new_unsupported(
                    "Expected control response".to_string(),
                ))
            }
        };

        match *control_response {
            ControlResponse::Connect(connect_resp) => match connect_resp.reply {
                ConnectResponseReply::Success(success) => Ok(success.ips),
                ConnectResponseReply::Failure(reason) => Err(Error::new_unsupported(format!(
                    "Connect denied: {:?}",
                    reason
                ))),
            },
            _ => Err(Error::new_unsupported(
                "Unexpected control response type".to_string(),
            )),
        }
    }

    pub async fn send_ip_packet(&mut self, packet: &[u8]) -> Result<(), Error> {
        if self.connection_state != ConnectionState::Connected {
            return Err(Error::new_unsupported("Not connected".to_string()));
        }

        let request = IpPacketRequest::new_data_request(packet.to_vec().into());
        self.send_ipr_request(request).await
    }

    pub async fn process_incoming(&mut self) -> Result<Vec<Bytes>, Error> {
        // TODO switch to framedreading?
        let mut buffer = vec![0u8; 65536];

        match tokio::time::timeout(Duration::from_secs(10), self.stream.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                debug!("Read {} bytes", n);

                let mut codec = ReconstructedMessageCodec {};
                let mut buf = BytesMut::from(&buffer[..n]);

                if let Ok(Some(reconstructed)) = codec.decode(&mut buf) {
                    match self
                        .listener
                        .handle_reconstructed_message(reconstructed)
                        .await
                    {
                        Ok(Some(MixnetMessageOutcome::IpPackets(packets))) => {
                            debug!("Extracted {} IP packets", packets.len());
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
                            debug!("Failed to handle message: {}", e);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    Ok(Vec::new())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    static INIT: Once = Once::new();

    fn init_logging() {
        INIT.call_once(|| {
            nym_bin_common::logging::setup_tracing_logger();
        });
    }

    #[tokio::test]
    async fn connect_to_ipr() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let socket = IpMixSocket::new().await?;

        let mut stream = socket.connect().await?;
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

        Ok(())
    }

    #[tokio::test]
    async fn send_ping() -> Result<(), Box<dyn std::error::Error>> {
        // TODO pull in https://github.com/nymtech/nym-vpn-client/blob/develop/nym-vpn-core/crates/nym-connection-monitor/src/packet_helpers.rs#L7-L42
        // TODO pull in https://github.com/nymtech/nym-vpn-client/blob/develop/nym-vpn-core/crates/nym-gateway-probe/src/icmp.rs#L25
        init_logging();

        let socket = IpMixSocket::new().await?;

        let mut stream = socket.connect().await?;
        let ip_pair = stream.connect_tunnel().await?;

        // send ping
        //

        Ok(())
    }
}
