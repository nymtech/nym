use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use nym_ip_packet_requests::{codec::MultiIpPacketCodec, v7, v8, IpPair};
use nym_sdk::mixnet::{AnonymousSenderTag, MixnetMessageSender, Recipient};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::TaskHandle;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, net::SocketAddr};
use tap::TapFallible;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio_util::codec::Decoder;

use crate::{
    config::Config,
    connected_client_handler,
    constants::{CLIENT_MIXNET_INACTIVITY_TIMEOUT, DISCONNECT_TIMER_INTERVAL},
    error::{IpPacketRouterError, Result},
    request_filter::{self},
    tun_listener,
    util::generate_new_ip,
    util::{
        create_message::create_input_message,
        parse_ip::{parse_packet, ParsedPacket},
    },
};

pub(crate) struct ConnectedClients {
    // The set of connected clients
    clients_ipv4_mapping: HashMap<Ipv4Addr, ConnectedClient>,
    clients_ipv6_mapping: HashMap<Ipv6Addr, ConnectedClient>,

    // Notify the tun listener when a new client connects or disconnects
    tun_listener_connected_client_tx: tokio::sync::mpsc::UnboundedSender<ConnectedClientEvent>,
}

impl ConnectedClients {
    pub(crate) fn new() -> (Self, tun_listener::ConnectedClientsListener) {
        let (connected_client_tx, connected_client_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                clients_ipv4_mapping: Default::default(),
                clients_ipv6_mapping: Default::default(),
                tun_listener_connected_client_tx: connected_client_tx,
            },
            tun_listener::ConnectedClientsListener::new(connected_client_rx),
        )
    }

    fn is_ip_connected(&self, ips: &IpPair) -> bool {
        self.clients_ipv4_mapping.contains_key(&ips.ipv4)
            || self.clients_ipv6_mapping.contains_key(&ips.ipv6)
    }

    fn get_client_from_ip_mut(&mut self, ip: &IpAddr) -> Option<&mut ConnectedClient> {
        match ip {
            IpAddr::V4(ip) => self.clients_ipv4_mapping.get_mut(ip),
            IpAddr::V6(ip) => self.clients_ipv6_mapping.get_mut(ip),
        }
    }

    fn is_client_connected(&self, client_id: &RequestSender) -> bool {
        self.clients_ipv4_mapping
            .values()
            .any(|client| client.client_id == *client_id)
    }

    fn lookup_ip_from_client_id(&self, client_id: &RequestSender) -> Option<IpPair> {
        self.clients_ipv4_mapping
            .iter()
            .find_map(|(ipv4, connected_client)| {
                if connected_client.client_id == *client_id {
                    Some(IpPair::new(*ipv4, connected_client.ipv6))
                } else {
                    None
                }
            })
    }

    fn lookup_client(&self, client_id: &RequestSender) -> Option<&ConnectedClient> {
        self.clients_ipv4_mapping
            .values()
            .find(|connected_client| connected_client.client_id == *client_id)
    }

    fn connect(
        &mut self,
        ips: IpPair,
        client_id: RequestSender,
        // nym_address: Recipient,
        forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        close_tx: tokio::sync::oneshot::Sender<()>,
        handle: tokio::task::JoinHandle<()>,
    ) {
        // The map of connected clients that the mixnet listener keeps track of. It monitors
        // activity and disconnects clients that have been inactive for too long.
        let client = ConnectedClient {
            client_id,
            ipv6: ips.ipv6,
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
            close_tx: Arc::new(CloseTx {
                client_id,
                inner: Some(close_tx),
            }),
            handle: Arc::new(handle),
        };
        log::info!("Inserting {} and {}", ips.ipv4, ips.ipv6);
        self.clients_ipv4_mapping.insert(ips.ipv4, client.clone());
        self.clients_ipv6_mapping.insert(ips.ipv6, client);
        // Send the connected client info to the tun listener, which will use it to forward packets
        // to the connected client handler.
        self.tun_listener_connected_client_tx
            .send(ConnectedClientEvent::Connect(Box::new(ConnectEvent {
                ips,
                forward_from_tun_tx,
            })))
            .tap_err(|err| {
                log::error!("Failed to send connected client event: {err}");
            })
            .ok();
    }

    async fn update_activity(&mut self, ips: &IpPair) -> Result<()> {
        if let Some(client) = self.clients_ipv4_mapping.get(&ips.ipv4) {
            *client.last_activity.write().await = std::time::Instant::now();
            Ok(())
        } else {
            Err(IpPacketRouterError::FailedToUpdateClientActivity)
        }
    }

    // Identify connected client handlers that have stopped without being told to stop
    fn get_finished_client_handlers(&mut self) -> Vec<(IpPair, RequestSender)> {
        self.clients_ipv4_mapping
            .iter_mut()
            .filter_map(|(ip, connected_client)| {
                if connected_client.handle.is_finished() {
                    Some((
                        IpPair::new(*ip, connected_client.ipv6),
                        connected_client.client_id,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    async fn get_inactive_clients(&mut self) -> Vec<(IpPair, RequestSender)> {
        let now = std::time::Instant::now();
        let mut ret = vec![];
        for (ip, connected_client) in self.clients_ipv4_mapping.iter() {
            if now.duration_since(*connected_client.last_activity.read().await)
                > CLIENT_MIXNET_INACTIVITY_TIMEOUT
            {
                ret.push((
                    IpPair::new(*ip, connected_client.ipv6),
                    connected_client.client_id,
                ))
            }
        }
        ret
    }

    fn disconnect_stopped_client_handlers(
        &mut self,
        stopped_clients: Vec<(IpPair, RequestSender)>,
    ) {
        for (ips, _) in &stopped_clients {
            log::info!("Disconnect stopped client: {ips}");
            self.clients_ipv4_mapping.remove(&ips.ipv4);
            self.clients_ipv6_mapping.remove(&ips.ipv6);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
        }
    }

    fn disconnect_inactive_clients(&mut self, inactive_clients: Vec<(IpPair, RequestSender)>) {
        for (ips, _) in &inactive_clients {
            log::info!("Disconnect inactive client: {ips}");
            self.clients_ipv4_mapping.remove(&ips.ipv4);
            self.clients_ipv6_mapping.remove(&ips.ipv6);
            self.tun_listener_connected_client_tx
                .send(ConnectedClientEvent::Disconnect(DisconnectEvent(*ips)))
                .tap_err(|err| {
                    log::error!("Failed to send disconnect event: {err}");
                })
                .ok();
        }
    }

    fn find_new_ip(&self) -> Option<IpPair> {
        generate_new_ip::find_new_ips(&self.clients_ipv4_mapping, &self.clients_ipv6_mapping)
    }
}

pub(crate) struct CloseTx {
    // pub(crate) nym_address: Recipient,
    pub(crate) client_id: RequestSender,
    // Send to connected clients listener to stop. This is option only because we need to take
    // ownership of it when the client is dropped.
    pub(crate) inner: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct ConnectedClient {
    // The nym address of the connected client that we are communicating with on the other side of
    // the mixnet
    // pub(crate) nym_address: Recipient,
    pub(crate) client_id: RequestSender,

    // The assigned IPv6 address of this client
    pub(crate) ipv6: Ipv6Addr,

    // Keep track of last activity so we can disconnect inactive clients
    pub(crate) last_activity: Arc<RwLock<std::time::Instant>>,

    pub(crate) close_tx: Arc<CloseTx>,

    // Handle for the connected client handler
    pub(crate) handle: Arc<tokio::task::JoinHandle<()>>,
}

impl ConnectedClient {
    async fn update_activity(&self) {
        *self.last_activity.write().await = std::time::Instant::now();
    }
}

impl Drop for CloseTx {
    fn drop(&mut self) {
        log::debug!("signal to close client: {}", self.client_id);
        if let Some(close_tx) = self.inner.take() {
            close_tx.send(()).ok();
        }
    }
}

type PacketHandleResult = Result<Option<VersionedResponse>>;

struct VersionedResponse {
    version: SupportedClientVersion,
    request_id: Option<u64>,
    reply_to: RequestSender,
    response: Response2,
}

#[derive(Debug, Clone)]
enum Response2 {
    StaticConnect(StaticConnectResponse),
    DynamicConnect(DynamicConnectResponse),
    Disconnect,
    Data,
    Pong,
    Health,
    Info(InfoResponse),
}

#[derive(Debug, Clone)]
enum StaticConnectResponse {
    Success,
    Failure(StaticConnectFailureReason),
}

#[derive(thiserror::Error, Debug, Clone)]
enum StaticConnectFailureReason {
    #[error("requested ip address is already in use")]
    RequestedIpAlreadyInUse,
    #[error("client already connected")]
    ClientAlreadyConnected,
    #[error("request timestamp is out of date")]
    OutOfDateTimestamp,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
enum DynamicConnectResponse {
    Success,
    Failure(DynamicConnectFailureReason),
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum DynamicConnectFailureReason {
    #[error("client already connected")]
    ClientAlreadyConnected,
    #[error("no available ip address")]
    NoAvailableIp,
    #[error("{0}")]
    Other(String),
}

impl From<VersionedResponse> for v7::response::IpPacketResponse {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response2::StaticConnect(inner) => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::StaticConnect(
                    v7::response::StaticConnectResponse {
                        request_id: response.request_id.unwrap(),
                        reply_to: response.reply_to.into_nym_address().unwrap(),
                        reply: match inner {
                            StaticConnectResponse::Success => {
                                v7::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v7::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response2::DynamicConnect(_) => {
                todo!();
            }
            Response2::Disconnect => todo!(),
            Response2::Data => todo!(),
            Response2::Pong => todo!(),
            Response2::Health => todo!(),
            Response2::Info(inner) => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Info(v7::response::InfoResponse {
                    request_id: response.request_id.unwrap(),
                    reply_to: response.reply_to.into_nym_address().unwrap(),
                    reply: inner.reply.into(),
                    level: inner.level.into(),
                }),
            },
        }
    }
}

impl From<VersionedResponse> for v8::response::IpPacketResponse {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response2::StaticConnect(inner) => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::StaticConnect(
                    v8::response::StaticConnectResponse {
                        request_id: response.request_id.unwrap(),
                        reply: match inner {
                            StaticConnectResponse::Success => {
                                v8::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v8::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response2::DynamicConnect(_) => {
                todo!();
            }
            Response2::Disconnect => todo!(),
            Response2::Data => todo!(),
            Response2::Pong => todo!(),
            Response2::Health => todo!(),
            Response2::Info(inner) => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Info(v8::response::InfoResponse {
                    request_id: response.request_id.unwrap(),
                    reply: inner.reply.into(),
                    level: inner.level.into(),
                }),
            },
        }
    }
}

impl From<StaticConnectFailureReason> for v7::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v7::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v7::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v7::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v7::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<StaticConnectFailureReason> for v8::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v8::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v8::response::StaticConnectFailureReason::ClientAlreadyConnected
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v8::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v8::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct InfoResponse {
    pub request_id: Option<u64>,
    pub reply: InfoResponseReply,
    pub level: InfoLevel,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum InfoResponseReply {
    #[error("{msg}")]
    Generic { msg: String },
    #[error(
        "version mismatch: response is v{request_version} and response is v{response_version}"
    )]
    VersionMismatch {
        request_version: u8,
        response_version: u8,
    },
    #[error("destination failed exit policy filter check: {dst}")]
    ExitPolicyFilterCheckFailed { dst: String },
}

impl From<InfoResponseReply> for v7::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v7::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v7::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v7::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<InfoResponseReply> for v8::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v8::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v8::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v8::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum InfoLevel {
    Info,
    Warn,
    Error,
}

impl From<InfoLevel> for v7::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v7::response::InfoLevel::Info,
            InfoLevel::Warn => v7::response::InfoLevel::Warn,
            InfoLevel::Error => v7::response::InfoLevel::Error,
        }
    }
}

impl From<InfoLevel> for v8::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v8::response::InfoLevel::Info,
            InfoLevel::Warn => v8::response::InfoLevel::Warn,
            InfoLevel::Error => v8::response::InfoLevel::Error,
        }
    }
}

//#[derive(Debug, Clone)]
//enum Response {
//    V7(v7::response::IpPacketResponse),
//    V8(v8::response::IpPacketResponse),
//}
//
//impl Response {
//    fn recipient(&self) -> Option<&Recipient> {
//        match self {
//            Response::V7(response) => response.recipient(),
//            Response::V8(response) => response.recipient(),
//        }
//    }
//
//    fn new_static_connect_success(
//        request_id: u64,
//        reply_to: Recipient,
//        client_version: SupportedClientVersion,
//    ) -> Self {
//        match client_version {
//            SupportedClientVersion::V7 => Response::V7(
//                v7::response::IpPacketResponse::new_static_connect_success(request_id, reply_to),
//            ),
//            SupportedClientVersion::V8 => Response::V8(
//                v8::response::IpPacketResponse::new_static_connect_success(request_id, reply_to),
//            ),
//        }
//    }
//
//    fn new_static_connect_failure(
//        request_id: u64,
//        reply_to: Recipient,
//        reason: StaticConnectFailureReason,
//        client_version: SupportedClientVersion,
//    ) -> Self {
//        match client_version {
//            SupportedClientVersion::V7 => {
//                Response::V7(v7::response::IpPacketResponse::new_static_connect_failure(
//                    request_id, reply_to, reason,
//                ))
//            }
//            SupportedClientVersion::V8 => {
//                Response::V8(v8::response::IpPacketResponse::new_static_connect_failure(
//                    request_id, reply_to, reason,
//                ))
//            }
//        }
//    }
//
//    fn new_dynamic_connect_success(
//        request_id: u64,
//        reply_to: Recipient,
//        ips: IpPair,
//        client_version: SupportedClientVersion,
//    ) -> Self {
//        match client_version {
//            SupportedClientVersion::V7 => {
//                Response::V7(v7::response::IpPacketResponse::new_dynamic_connect_success(
//                    request_id, reply_to, ips,
//                ))
//            }
//            SupportedClientVersion::V8 => {
//                Response::V8(v8::response::IpPacketResponse::new_dynamic_connect_success(
//                    request_id, reply_to, ips,
//                ))
//            }
//        }
//    }
//
//    fn new_dynamic_connect_failure(
//        request_id: u64,
//        reply_to: Recipient,
//        reason: DynamicConnectFailureReason,
//        client_version: SupportedClientVersion,
//    ) -> Self {
//        match client_version {
//            SupportedClientVersion::V7 => {
//                Response::V7(v7::response::IpPacketResponse::new_dynamic_connect_failure(
//                    request_id, reply_to, reason,
//                ))
//            }
//            SupportedClientVersion::V8 => {
//                Response::V8(v8::response::IpPacketResponse::new_dynamic_connect_failure(
//                    request_id, reply_to, reason,
//                ))
//            }
//        }
//    }
//
//    fn new_data_info_response(
//        reply_to: Recipient,
//        reply: InfoResponseReply,
//        level: InfoLevel,
//        client_version: SupportedClientVersion,
//    ) -> Self {
//        match client_version {
//            SupportedClientVersion::V7 => Response::V7(
//                v7::response::IpPacketResponse::new_data_info_response(reply_to, reply, level),
//            ),
//            SupportedClientVersion::V8 => Response::V8(
//                v8::response::IpPacketResponse::new_data_info_response(reply_to, reply, level),
//            ),
//        }
//    }
//
//    fn to_bytes(&self) -> Result<Vec<u8>> {
//        match self {
//            Response::V7(response) => response.to_bytes(),
//            Response::V8(response) => response.to_bytes(),
//        }
//        .map_err(|err| {
//            log::error!("Failed to serialize response packet");
//            IpPacketRouterError::FailedToSerializeResponsePacket { source: err }
//        })
//    }
//}

#[cfg(not(target_os = "linux"))]
type TunDevice = crate::non_linux_dummy::DummyDevice;

#[cfg(target_os = "linux")]
type TunDevice = tokio_tun::Tun;

// #[cfg(target_os = "linux")]
pub(crate) struct MixnetListener {
    // The configuration for the mixnet listener
    pub(crate) _config: Config,

    // The request filter that we use to check if a packet should be forwarded
    pub(crate) request_filter: request_filter::RequestFilter,

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
    // Receving a static connect request from a client with an IP provided that we assign to them,
    // if it's available. If it's not available, we send a failure response.
    async fn on_static_connect_request(
        &mut self,
        // from: RequestSender,
        connect_request: StaticConnectRequest2,
        version: SupportedClientVersion,
        // sender_tag: Option<AnonymousSenderTag>,
    ) -> PacketHandleResult {
        //log::info!(
        //    "Received static connect request from {}",
        //    connect_request.signed_by
        //);
        //sender_tag.inspect(|tag| log::info!("Connection is using SURBs: {tag}"));

        let request_id = connect_request.request_id;
        let requested_ips = connect_request.ips;
        // let reply_to = connect_request.signed_by;
        let buffer_timeout = connect_request
            .buffer_timeout
            .map(|timeout| Duration::from_millis(timeout))
            .unwrap_or(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);

        // Check that the IP is available in the set of connected clients
        let is_ip_taken = self.connected_clients.is_ip_connected(&requested_ips);

        // Check that the client_id address isn't already registered
        let is_client_id_taken = self
            .connected_clients
            .is_client_connected(&connect_request.sent_by);

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
                Response2::StaticConnect(StaticConnectResponse::Success)
                //Ok(Some(Response::new_static_connect_success(
                //    request_id,
                //    reply_to,
                //    client_version,
                //)))
            }
            (false, false) => {
                log::info!("Connecting a new client");

                // Spawn the ConnectedClientHandler for the new client
                let (forward_from_tun_tx, close_tx, handle) =
                    connected_client_handler::ConnectedClientHandler::start(
                        // reply_to,
                        // sender_tag,
                        connect_request.sent_by,
                        buffer_timeout,
                        version,
                        self.mixnet_client.split_sender(),
                    );

                // Register the new client in the set of connected clients
                self.connected_clients.connect(
                    requested_ips,
                    // reply_to,
                    connect_request.sent_by,
                    forward_from_tun_tx,
                    close_tx,
                    handle,
                );
                Response2::StaticConnect(StaticConnectResponse::Success)
                //Ok(Some(Response::new_static_connect_success(
                //    request_id,
                //    reply_to,
                //    client_version,
                //)))
            }
            (true, false) => {
                log::info!("Requested IP is not available");
                Response2::StaticConnect(StaticConnectResponse::Failure(
                    StaticConnectFailureReason::RequestedIpAlreadyInUse,
                ))
                //Ok(Some(Response::new_static_connect_failure(
                //    request_id,
                //    reply_to,
                //    StaticConnectFailureReason::RequestedIpAlreadyInUse,
                //    client_version,
                //)))
            }
            (false, true) => {
                log::info!("Nym address is already registered");
                Response2::StaticConnect(StaticConnectResponse::Failure(
                    StaticConnectFailureReason::ClientAlreadyConnected,
                ))
                //Ok(Some(Response::new_static_connect_failure(
                //    request_id,
                //    reply_to,
                //    StaticConnectFailureReason::RequestedNymAddressAlreadyInUse,
                //    client_version,
                //)))
            }
        };

        Ok(Some(VersionedResponse {
            version,
            request_id: Some(request_id),
            reply_to: connect_request.sent_by,
            response,
        }))
    }

    async fn on_dynamic_connect_request(
        &mut self,
        connect_request: DynamicConnectRequest2,
        version: SupportedClientVersion,
    ) -> PacketHandleResult {
        //log::info!(
        //    "Received dynamic connect request from {sender_address}",
        //    sender_address = connect_request.reply_to
        //);
        //sender_tag.inspect(|tag| log::info!("Connection is using SURBs: {tag}"));

        let request_id = connect_request.request_id;
        let reply_to = connect_request.sent_by;
        // TODO: ignoring reply_to_avg_mix_delays for now
        let buffer_timeout = connect_request
            .buffer_timeout
            .map(|timeout| Duration::from_millis(timeout))
            .unwrap_or(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);

        if self.connected_clients.is_client_connected(&reply_to) {
            return Ok(Some(VersionedResponse {
                version,
                request_id: Some(request_id),
                reply_to,
                response: Response2::DynamicConnect(DynamicConnectResponse::Failure(
                    DynamicConnectFailureReason::ClientAlreadyConnected,
                )),
            }));
        }

        let Some(new_ips) = self.connected_clients.find_new_ip() else {
            log::info!("No available IP address");
            return Ok(Some(VersionedResponse {
                version,
                request_id: Some(request_id),
                reply_to,
                response: Response2::DynamicConnect(DynamicConnectResponse::Failure(
                    DynamicConnectFailureReason::NoAvailableIp,
                )),
            }));
        };

        // Spawn the ConnectedClientHandler for the new client
        let (forward_from_tun_tx, close_tx, handle) =
            connected_client_handler::ConnectedClientHandler::start(
                reply_to,
                buffer_timeout,
                version,
                self.mixnet_client.split_sender(),
            );

        // Register the new client in the set of connected clients
        self.connected_clients
            .connect(new_ips, reply_to, forward_from_tun_tx, close_tx, handle);
        Ok(Some(VersionedResponse {
            version,
            request_id: Some(request_id),
            reply_to,
            response: Response2::DynamicConnect(DynamicConnectResponse::Success),
        }))
    }

    fn on_disconnect_request(
        &self,
        _disconnect_request: DisconnectRequest2,
        _client_version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::info!("Received disconnect request: not implemented, dropping");
        Ok(None)
    }

    async fn handle_packet(
        &mut self,
        ip_packet: &Bytes,
        version: SupportedClientVersion,
    ) -> PacketHandleResult {
        log::trace!("Received data request");

        // We don't forward packets that we are not able to parse. BUT, there might be a good
        // reason to still forward them.
        //
        // For example, if we are running in a mode where we are only supposed to forward
        // packets to a specific destination, we might want to forward them anyway.
        //
        // TODO: look into this
        let ParsedPacket {
            packet_type,
            src_addr,
            dst_addr,
            dst,
        } = parse_packet(ip_packet)?;

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
                log::info!("Denied filter check: {dst}");
                Ok(Some(VersionedResponse {
                    version,
                    request_id: None,
                    reply_to: connected_client.client_id,
                    response: Response2::Info(InfoResponse {
                        request_id: None,
                        reply: InfoResponseReply::ExitPolicyFilterCheckFailed {
                            dst: dst.to_string(),
                        },
                        level: InfoLevel::Warn,
                    }),
                }))
                //Ok(Some(Response::new_data_info_response(
                //    connected_client.nym_address,
                //    InfoResponseReply::ExitPolicyFilterCheckFailed {
                //        dst: dst.to_string(),
                //    },
                //    InfoLevel::Warn,
                //    client_version,
                //)))
            }
        } else {
            // If the client is not connected, just drop the packet silently
            log::debug!("dropping packet from mixnet: no registered client for packet with source: {src_addr}");
            Ok(None)
        }
    }

    async fn on_data_request(
        &mut self,
        data_request: DataRequest2,
        client_version: SupportedClientVersion,
    ) -> Result<Vec<PacketHandleResult>> {
        let mut responses = Vec::new();
        let mut decoder = MultiIpPacketCodec::new(nym_ip_packet_requests::codec::BUFFER_TIMEOUT);
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&data_request.ip_packets);
        while let Ok(Some(packet)) = decoder.decode(&mut bytes) {
            let result = self.handle_packet(&packet, client_version).await;
            responses.push(result);
        }
        Ok(responses)
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
        let (request, version) = match deserialize_request(&reconstructed) {
            Err(IpPacketRouterError::InvalidPacketVersion(version)) => {
                log::debug!("Received packet with invalid version: v{version}");
                return Ok(vec![self.on_version_mismatch(version, &reconstructed)]);
            }
            req => req,
        }?;

        log::debug!("Received request: {request}");

        // Verify signature
        request
            .verify()
            .inspect_err(|err| log::error!("Failed to verify request signature: {err}"))?;

        let request = IpPacketRequest2::from(request);

        match request.data {
            IpPacketRequestData2::StaticConnect(connect_request) => Ok(vec![
                self.on_static_connect_request(connect_request, version)
                    .await,
            ]),
            IpPacketRequestData2::DynamicConnect(connect_request) => Ok(vec![
                self.on_dynamic_connect_request(connect_request, version)
                    .await,
            ]),
            IpPacketRequestData2::Disconnect(disconnect_request) => {
                Ok(vec![self.on_disconnect_request(disconnect_request, version)])
            }
            IpPacketRequestData2::Data(data_request) => {
                self.on_data_request(data_request, version).await
            }
            IpPacketRequestData2::Ping(_) => {
                log::info!("Received ping request: not implemented, dropping");
                Ok(vec![])
            }
            IpPacketRequestData2::Health(_) => {
                log::info!("Received health request: not implemented, dropping");
                Ok(vec![])
            }
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
        let send_to = response.reply_to;
        let response_packet = match response.version {
            SupportedClientVersion::V7 => v7::response::IpPacketResponse::from(response)
                .to_bytes()
                .unwrap(),
            SupportedClientVersion::V8 => v8::response::IpPacketResponse::from(response)
                .to_bytes()
                .unwrap(),
        };

        // Convert from VersionedResponse to Response
        // let request_id = todo!();
        // let reply_to = todo!();

        // let response_packet = response.to_bytes()?;
        let input_message = create_input_message(&send_to, response_packet);
        self.mixnet_client
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })
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

fn deserialize_request(
    reconstructed: &ReconstructedMessage,
) -> Result<(IpPacketRequest, SupportedClientVersion)> {
    let request_version = *reconstructed
        .message
        .first()
        .ok_or(IpPacketRouterError::EmptyPacket)?;

    // Check version of the request and convert to the latest version if necessary
    let request = match request_version {
        7 => v7::request::IpPacketRequest::from_reconstructed_message(reconstructed)
            .map(IpPacketRequest::from)
            .map_err(|source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source }),
        8 => {
            let sender_tag = reconstructed
                .sender_tag
                .ok_or(IpPacketRouterError::EmptyPacket)?;
            v8::request::IpPacketRequest::from_reconstructed_message(reconstructed)
                .map(|r| IpPacketRequest::from((r, sender_tag)))
                .map_err(|source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source })
        }
        _ => {
            log::info!("Received packet with invalid version: v{request_version}");
            Err(IpPacketRouterError::InvalidPacketVersion(request_version))
        }
    };

    let Some(request_version) = SupportedClientVersion::new(request_version) else {
        return Err(IpPacketRouterError::InvalidPacketVersion(request_version));
    };

    // Tag the request with the version of the request
    request.map(|r| (r, request_version))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IpPacketRequest2 {
    pub version: u8,
    pub data: IpPacketRequestData2,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequestData2 {
    StaticConnect(StaticConnectRequest2),
    DynamicConnect(DynamicConnectRequest2),
    Disconnect(DisconnectRequest2),
    Data(DataRequest2),
    Ping(PingRequest2),
    Health(HealthRequest2),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StaticConnectRequest2 {
    request_id: u64,
    sent_by: RequestSender,
    ips: IpPair,
    buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DynamicConnectRequest2 {
    request_id: u64,
    sent_by: RequestSender,
    buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisconnectRequest2 {
    request_id: u64,
    sent_by: RequestSender,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataRequest2 {
    ip_packets: bytes::Bytes,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PingRequest2 {
    request_id: u64,
    sent_by: RequestSender,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HealthRequest2 {
    request_id: u64,
    sent_by: RequestSender,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RequestSender {
    NymAddress(Recipient),
    SenderTag(AnonymousSenderTag),
}
impl RequestSender {
    fn into_nym_address(&self) -> Option<Recipient> {
        match self {
            RequestSender::NymAddress(nym_address) => Some(*nym_address),
            RequestSender::SenderTag(_) => None,
        }
    }

    fn into_sender_tag(&self) -> Option<AnonymousSenderTag> {
        match self {
            RequestSender::NymAddress(_) => None,
            RequestSender::SenderTag(tag) => Some(*tag),
        }
    }
}

impl fmt::Display for RequestSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestSender::NymAddress(nym_address) => write!(f, "{nym_address}"),
            RequestSender::SenderTag(tag) => write!(f, "{tag}"),
        }
    }
}

impl From<v7::request::IpPacketRequest> for IpPacketRequest2 {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        Self {
            version: 7,
            data: match request.data {
                v7::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData2::StaticConnect(StaticConnectRequest2 {
                        request_id: inner.request.request_id,
                        sent_by: RequestSender::NymAddress(inner.request.reply_to),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v7::request::IpPacketRequestData::DynamicConnect(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Disconnect(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData2::Data(DataRequest2 {
                        ip_packets: inner.ip_packets,
                    })
                }
                v7::request::IpPacketRequestData::Ping(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Health(_) => {
                    todo!();
                }
            },
        }
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for IpPacketRequest2 {
    fn from((request, sender_tag): (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        Self {
            version: 8,
            data: match request.data {
                v8::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData2::StaticConnect(StaticConnectRequest2 {
                        request_id: inner.request.request_id,
                        sent_by: RequestSender::SenderTag(sender_tag),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v8::request::IpPacketRequestData::DynamicConnect(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Disconnect(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData2::Data(DataRequest2 {
                        ip_packets: inner.ip_packets,
                    })
                }
                v8::request::IpPacketRequestData::Ping(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Health(_) => {
                    todo!();
                }
            },
        }
    }
}

impl From<IpPacketRequest> for IpPacketRequest2 {
    fn from(request: IpPacketRequest) -> Self {
        match request {
            IpPacketRequest::V7(request) => request.into(),
            IpPacketRequest::V8(request) => request.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequest {
    V7(v7::request::IpPacketRequest),
    V8((v8::request::IpPacketRequest, AnonymousSenderTag)),
}

impl IpPacketRequest {
    pub(crate) fn version(&self) -> u8 {
        match self {
            IpPacketRequest::V7(_) => 7,
            IpPacketRequest::V8(_) => 8,
        }
    }

    pub(crate) fn verify(&self) -> Result<()> {
        match self {
            IpPacketRequest::V7(request) => request.verify(),
            IpPacketRequest::V8(request) => request.0.verify(),
        }
        .map_err(|err| IpPacketRouterError::FailedToVerifyRequest { source: err })
    }
}

impl fmt::Display for IpPacketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpPacketRequest::V7(request) => write!(f, "{request}"),
            IpPacketRequest::V8((request, _)) => write!(f, "{request}"),
        }
    }
}

impl From<v7::request::IpPacketRequest> for IpPacketRequest {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        IpPacketRequest::V7(request)
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for IpPacketRequest {
    fn from(request: (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        IpPacketRequest::V8(request)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SupportedClientVersion {
    V7,
    V8,
}

impl SupportedClientVersion {
    fn new(request_version: u8) -> Option<Self> {
        match request_version {
            7 => Some(SupportedClientVersion::V7),
            8 => Some(SupportedClientVersion::V8),
            _ => None,
        }
    }
}

impl SupportedClientVersion {
    fn into_u8(self) -> u8 {
        match self {
            SupportedClientVersion::V7 => 7,
            SupportedClientVersion::V8 => 8,
        }
    }
}

//fn verify_signed_request(request: &impl SignedRequest) -> Result<()> {
//    request
//        .verify()
//        .map_err(|err| IpPacketRouterError::FailedToVerifyRequest { source: err })
//}

pub(crate) enum ConnectedClientEvent {
    Disconnect(DisconnectEvent),
    Connect(Box<ConnectEvent>),
}

pub(crate) struct DisconnectEvent(pub(crate) IpPair);

pub(crate) struct ConnectEvent {
    pub(crate) ips: IpPair,
    pub(crate) forward_from_tun_tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}
