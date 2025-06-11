// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::StreamExt;
use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::mixnet::MixnetMessageSender;
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use std::{net::SocketAddr, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::FramedRead;

use crate::{
    clients::{ConnectedClientHandler, ConnectedClients},
    config::Config,
    constants::DISCONNECT_TIMER_INTERVAL,
    error::{IpPacketRouterError, Result},
    messages::{
        request::{
            ControlRequest, DataRequest, DisconnectRequest, DynamicConnectRequest, HealthRequest,
            IpPacketRequest, PingRequest, StaticConnectRequest,
        },
        response::{
            DisconnectFailureReason, DisconnectResponse, DynamicConnectFailureReason,
            DynamicConnectSuccess, HealthResponse, InfoLevel, InfoResponse, InfoResponseReply,
            Response, StaticConnectFailureReason, StaticConnectResponse, VersionedResponse,
        },
        ClientVersion,
    },
    request_filter::RequestFilter,
    util::parse_ip::ParsedPacket,
};

#[cfg(not(target_os = "linux"))]
type TunDevice = crate::non_linux_dummy::DummyDevice;

#[cfg(target_os = "linux")]
type TunDevice = tokio_tun::Tun;

// #[cfg(target_os = "linux")]
pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) _config: Config,

    // The request filter that we use to check if a packet should be forwarded
    pub(crate) request_filter: RequestFilter,

    // The TUN device that we use to send and receive packets from the internet
    pub(crate) tun_writer: tokio::io::WriteHalf<TunDevice>,

    // The mixnet client that we use to send and receive packets from the mixnet
    pub(crate) mixnet_client: nym_sdk::mixnet::MixnetClient,

    // The task handle for the main loop
    pub(crate) task_handle: TaskHandle,

    // The map of connected clients that the mixnet listener keeps track of. It monitors
    // activity and disconnects clients that have been inactive for too long.
    pub(crate) connected_clients: ConnectedClients,
}

// #[cfg(target_os = "linux")]
impl MixnetListener {
    async fn handle_packet(
        &mut self,
        ip_packet: &[u8],
        version: ClientVersion,
    ) -> PacketHandleResult {
        log::trace!("Received data request");

        // We don't forward packets that we are not able to parse. BUT, there might be a good
        // reason to still forward them.
        //
        // TODO: look into this
        let ParsedPacket {
            packet_type,
            src_addr,
            dst_addr,
            dst,
        } = crate::util::parse_ip::parse_packet(ip_packet)?;

        let dst_str = dst.map_or(dst_addr.to_string(), |dst| dst.to_string());
        log::debug!("Received packet: {packet_type}: {src_addr} -> {dst_str}");

        if let Some(connected_client) = self.connected_clients.get_client_from_ip_mut(&src_addr) {
            // Keep track of activity so we can disconnect inactive clients
            connected_client.update_activity().await;

            // For packets without a port, use 0.
            let dst = dst.unwrap_or_else(|| SocketAddr::new(dst_addr, 0));

            // Filter check
            if self.request_filter.check_address(&dst).await {
                // Forward the packet to the TUN device where it will be routed out to the internet
                self.tun_writer
                    .write_all(ip_packet)
                    .await
                    .map_err(|_| IpPacketRouterError::FailedToWritePacketToTun)?;
                Ok(None)
            } else {
                log::debug!("Denied filter check: {dst}");
                Ok(Some(VersionedResponse {
                    version,
                    reply_to: connected_client.client_id.clone(),
                    response: Response::Info {
                        request_id: 0,
                        reply: InfoResponse {
                            reply: InfoResponseReply::ExitPolicyFilterCheckFailed {
                                dst: dst.to_string(),
                            },
                            level: InfoLevel::Warn,
                        },
                    },
                }))
            }
        } else {
            // If the client is not connected, just drop the packet silently
            log::debug!("dropping packet from mixnet: no registered client for packet with source: {src_addr}");
            Ok(None)
        }
    }

    async fn on_data_request(
        &mut self,
        data_request: DataRequest,
    ) -> Result<Vec<PacketHandleResult>> {
        let mut responses = Vec::new();
        let decoder = MultiIpPacketCodec::new();
        let mut framed_reader = FramedRead::new(data_request.ip_packets.as_ref(), decoder);

        while let Some(Ok(packet)) = framed_reader.next().await {
            let result = self
                .handle_packet(packet.as_bytes(), data_request.version)
                .await;
            responses.push(result);
        }

        Ok(responses)
    }

    // Receving a static connect request from a client with an IP provided that we assign to them,
    // if it's available. If it's not available, we send a failure response.
    async fn on_static_connect_request(
        &mut self,
        connect_request: StaticConnectRequest,
    ) -> PacketHandleResult {
        log::info!(
            "Received static connect request from {}",
            connect_request.sent_by
        );

        let version = connect_request.version;
        let sent_by = connect_request.sent_by;
        let request_id = connect_request.request_id;
        let requested_ips = connect_request.ips;
        let buffer_timeout = connect_request
            .buffer_timeout
            .map(Duration::from_millis)
            .unwrap_or(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);

        // Check that the IP is available in the set of connected clients
        let is_ip_taken = self.connected_clients.is_ip_connected(&requested_ips);

        // Check that the client_id address isn't already registered
        let is_client_id_taken = self.connected_clients.is_client_connected(&sent_by);

        let response = match (is_ip_taken, is_client_id_taken) {
            (true, true) => {
                log::info!("Connecting an already connected client");
                if self
                    .connected_clients
                    .update_activity(&requested_ips)
                    .await
                    .is_err()
                {
                    log::error!("Failed to update activity for client");
                };
                Response::StaticConnect {
                    request_id,
                    reply: StaticConnectResponse::Success,
                }
            }
            (false, false) => {
                log::info!("Connecting a new client");

                // Spawn the ConnectedClientHandler for the new client
                let (forward_from_tun_tx, close_tx, handle) = ConnectedClientHandler::start(
                    sent_by.clone(),
                    buffer_timeout,
                    version,
                    self.mixnet_client.split_sender(),
                );

                // Register the new client in the set of connected clients
                self.connected_clients.connect(
                    requested_ips,
                    sent_by.clone(),
                    forward_from_tun_tx,
                    close_tx,
                    handle,
                );
                Response::StaticConnect {
                    request_id,
                    reply: StaticConnectResponse::Success,
                }
            }
            (true, false) => {
                log::info!("Requested IP is not available");
                Response::StaticConnect {
                    request_id,
                    reply: StaticConnectFailureReason::RequestedIpAlreadyInUse.into(),
                }
            }
            (false, true) => {
                log::info!("Nym address is already registered");
                Response::StaticConnect {
                    request_id,
                    reply: StaticConnectFailureReason::ClientAlreadyConnected.into(),
                }
            }
        };

        Ok(Some(VersionedResponse {
            version,
            reply_to: sent_by,
            response,
        }))
    }

    fn on_dynamic_connect_request(
        &mut self,
        connect_request: DynamicConnectRequest,
    ) -> PacketHandleResult {
        log::info!(
            "Received dynamic connect request from {}",
            connect_request.sent_by
        );

        let version = connect_request.version;
        let request_id = connect_request.request_id;
        let reply_to = connect_request.sent_by;
        let buffer_timeout = connect_request
            .buffer_timeout
            .map(Duration::from_millis)
            .unwrap_or(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);

        if let Some(ips) = self.connected_clients.lookup_ip_from_client_id(&reply_to) {
            log::debug!("Reconnecting to the previous session");
            return Ok(Some(VersionedResponse {
                version,
                reply_to,
                response: Response::DynamicConnect {
                    request_id,
                    reply: DynamicConnectSuccess { ips }.into(),
                },
            }));
        }

        let Some(new_ips) = self.connected_clients.find_new_ip() else {
            log::info!("No available IP address");
            return Ok(Some(VersionedResponse {
                version,
                reply_to,
                response: Response::DynamicConnect {
                    request_id,
                    reply: DynamicConnectFailureReason::NoAvailableIp.into(),
                },
            }));
        };

        // Spawn the ConnectedClientHandler for the new client
        let (forward_from_tun_tx, close_tx, handle) = ConnectedClientHandler::start(
            reply_to.clone(),
            buffer_timeout,
            version,
            self.mixnet_client.split_sender(),
        );

        // Register the new client in the set of connected clients
        self.connected_clients.connect(
            new_ips,
            reply_to.clone(),
            forward_from_tun_tx,
            close_tx,
            handle,
        );

        Ok(Some(VersionedResponse {
            version,
            reply_to,
            response: Response::DynamicConnect {
                request_id,
                reply: DynamicConnectSuccess { ips: new_ips }.into(),
            },
        }))
    }

    fn on_disconnect_request(
        &mut self,
        disconnect_request: DisconnectRequest,
    ) -> PacketHandleResult {
        log::info!(
            "Received disconnect request from {}",
            disconnect_request.sent_by
        );

        let version = disconnect_request.version;
        let request_id = disconnect_request.request_id;
        let client_id = disconnect_request.sent_by;

        // Check if the client is connected
        if !self.connected_clients.is_client_connected(&client_id) {
            log::info!("Client {} is not connected, cannot disconnect", client_id);
            return Ok(Some(VersionedResponse {
                version,
                reply_to: client_id,
                response: Response::Disconnect {
                    request_id,
                    reply: DisconnectResponse::Failure(DisconnectFailureReason::ClientNotConnected),
                },
            }));
        }

        // Disconnect the client
        log::info!("Disconnecting client {}", client_id);
        self.connected_clients.disconnect_client(&client_id);

        Ok(Some(VersionedResponse {
            version,
            reply_to: client_id,
            response: Response::Disconnect {
                request_id,
                reply: DisconnectResponse::Success,
            },
        }))
    }

    fn on_ping_request(&self, ping_request: PingRequest) -> PacketHandleResult {
        Ok(Some(VersionedResponse {
            version: ping_request.version,
            reply_to: ping_request.sent_by,
            response: Response::Pong {
                request_id: ping_request.request_id,
            },
        }))
    }

    fn on_health_request(&self, health_request: HealthRequest) -> PacketHandleResult {
        Ok(Some(VersionedResponse {
            version: health_request.version,
            reply_to: health_request.sent_by,
            response: Response::Health {
                request_id: health_request.request_id,
                reply: Box::new(HealthResponse {
                    build_info: nym_bin_common::bin_info_owned!(),
                    routable: None,
                }),
            },
        }))
    }

    async fn on_control_request(&mut self, control_request: ControlRequest) -> PacketHandleResult {
        match control_request {
            ControlRequest::StaticConnect(r) => self.on_static_connect_request(r).await,
            ControlRequest::DynamicConnect(r) => self.on_dynamic_connect_request(r),
            ControlRequest::Disconnect(r) => self.on_disconnect_request(r),
            ControlRequest::Ping(r) => self.on_ping_request(r),
            ControlRequest::Health(r) => self.on_health_request(r),
        }
    }

    fn on_version_mismatch(
        &self,
        _version: u8,
        _reconstructed: &ReconstructedMessage,
    ) -> PacketHandleResult {
        // Just drop it. In the future we might want to return a response here, if for example
        // the client is connecting with a version that is older than the currently supported
        // ones.
        Ok(None)
    }

    async fn on_reconstructed_message(
        &mut self,
        reconstructed: ReconstructedMessage,
    ) -> Result<Vec<PacketHandleResult>> {
        log::debug!(
            "Received message with sender_tag: {}",
            reconstructed
                .sender_tag
                .map(|tag| tag.to_string())
                .unwrap_or("missing".to_owned())
        );

        // First deserialize the request
        let request = match IpPacketRequest::try_from(&reconstructed) {
            Err(IpPacketRouterError::InvalidPacketVersion(version)) => {
                log::debug!("Received packet with invalid version: v{version}");
                return Ok(vec![self.on_version_mismatch(version, &reconstructed)]);
            }
            req => req,
        }?;

        log::debug!("Received request: {request}");

        match request {
            IpPacketRequest::Data(request) => self.on_data_request(request).await,
            IpPacketRequest::Control(request) => Ok(vec![self.on_control_request(request).await]),
        }
    }

    async fn handle_disconnect_timer(&mut self) {
        let stopped_clients = self.connected_clients.get_finished_client_handlers();
        let inactive_clients = self.connected_clients.get_inactive_clients().await;

        // TODO: Send disconnect responses to all disconnected clients
        //for (ip, nym_address) in stopped_clients.iter().chain(disconnected_clients.iter()) {
        //    let response = IpPacketResponse::new_unrequested_disconnect(...)
        //    if let Err(err) = self.handle_response(response).await {
        //        log::error!("Failed to send disconnect response: {err}");
        //    }
        //}

        self.connected_clients
            .disconnect_stopped_client_handlers(stopped_clients);
        self.connected_clients
            .disconnect_inactive_clients(inactive_clients);
    }

    // When an incoming mixnet message triggers a response that we send back, such as during
    // connect handshake.
    async fn handle_response(&self, response: VersionedResponse) -> Result<()> {
        let send_to = response.reply_to.clone();
        let response_bytes = response.try_into_bytes()?;
        let input_message =
            crate::util::create_message::create_input_message(&send_to, response_bytes);

        self.mixnet_client.send(input_message).await.map_err(|err| {
            IpPacketRouterError::FailedToSendPacketToMixnet {
                source: Box::new(err),
            }
        })
    }

    // A single incoming request can trigger multiple responses, such as when data requests contain
    // multiple IP packets.
    async fn handle_responses(&self, responses: Vec<PacketHandleResult>) {
        for response in responses {
            match response {
                Ok(Some(response)) => {
                    if let Err(err) = self.handle_response(response).await {
                        log::error!("Mixnet listener failed to handle response: {err}");
                    }
                }
                Ok(None) => {
                    continue;
                }
                Err(err) => {
                    log::error!("Error handling mixnet message: {err}");
                }
            }
        }
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut task_client = self.task_handle.fork("main_loop");
        let mut disconnect_timer = tokio::time::interval(DISCONNECT_TIMER_INTERVAL);

        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpPacketRouter [main loop]: received shutdown");
                },
                _ = disconnect_timer.tick() => {
                    self.handle_disconnect_timer().await;
                },
                msg = self.mixnet_client.next() => {
                    if let Some(msg) = msg {
                        match self.on_reconstructed_message(msg).await {
                            Ok(responses) => self.handle_responses(responses).await,
                            Err(err) => {
                                log::error!("Error handling reconstructed mixnet message: {err}");
                            }

                        };
                    } else {
                        log::trace!("IpPacketRouter [main loop]: stopping since channel closed");
                        break;
                    };
                },

            }
        }
        log::debug!("IpPacketRouter: stopping");
        Ok(())
    }
}

pub(crate) type PacketHandleResult = Result<Option<VersionedResponse>>;
