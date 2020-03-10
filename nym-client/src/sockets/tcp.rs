use crate::client::received_buffer::ReceivedBufferResponse;
use crate::client::topology_control::TopologyAccessor;
use crate::client::InputMessage;
use futures::channel::{mpsc, oneshot};
use futures::future::FutureExt;
use futures::io::Error;
use futures::SinkExt;
use log::*;
use sphinx::route::{Destination, DestinationAddressBytes};
use std::convert::TryFrom;
use std::io;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use topology::NymTopology;

const SEND_REQUEST_PREFIX: u8 = 1;
const FETCH_REQUEST_PREFIX: u8 = 2;
const GET_CLIENTS_REQUEST_PREFIX: u8 = 3;
const OWN_DETAILS_REQUEST_PREFIX: u8 = 4;

#[derive(Debug)]
pub enum TCPSocketError {
    FailedToStartSocketError,
    UnknownSocketError,
    IncompleteDataError,
    UnknownRequestError,
}

impl From<io::Error> for TCPSocketError {
    fn from(err: Error) -> Self {
        use TCPSocketError::*;
        match err.kind() {
            io::ErrorKind::ConnectionRefused => FailedToStartSocketError,
            io::ErrorKind::ConnectionReset => FailedToStartSocketError,
            io::ErrorKind::ConnectionAborted => FailedToStartSocketError,
            io::ErrorKind::NotConnected => FailedToStartSocketError,

            io::ErrorKind::AddrInUse => FailedToStartSocketError,
            io::ErrorKind::AddrNotAvailable => FailedToStartSocketError,
            _ => UnknownSocketError,
        }
    }
}

enum ClientRequest {
    Send {
        message: Vec<u8>,
        recipient_address: DestinationAddressBytes,
    },
    Fetch,
    GetClients,
    OwnDetails,
}

impl TryFrom<&[u8]> for ClientRequest {
    type Error = TCPSocketError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        use TCPSocketError::*;
        if data.is_empty() {
            return Err(IncompleteDataError);
        }

        match data[0] {
            SEND_REQUEST_PREFIX => parse_send_request(data),
            FETCH_REQUEST_PREFIX => Ok(ClientRequest::Fetch),
            GET_CLIENTS_REQUEST_PREFIX => Ok(ClientRequest::GetClients),
            OWN_DETAILS_REQUEST_PREFIX => Ok(ClientRequest::OwnDetails),
            _ => Err(UnknownRequestError),
        }
    }
}

fn parse_send_request(data: &[u8]) -> Result<ClientRequest, TCPSocketError> {
    if data.len() < 1 + 32 + 1 {
        // make sure it has the prefix, destination and at least single byte of data
        return Err(TCPSocketError::IncompleteDataError);
    }

    let mut recipient_address = [0u8; 32];
    recipient_address.copy_from_slice(&data[1..33]);

    let message = data[33..].to_vec();

    Ok(ClientRequest::Send {
        message,
        recipient_address: DestinationAddressBytes::from_bytes(recipient_address),
    })
}

impl ClientRequest {
    async fn handle_send(
        msg: Vec<u8>,
        recipient_address: DestinationAddressBytes,
        mut input_tx: mpsc::UnboundedSender<InputMessage>,
    ) -> ServerResponse {
        trace!("sending to: {:?}, msg: {:?}", recipient_address, msg);
        if msg.len() > sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH {
            return ServerResponse::Error {
                message: format!(
                    "too long message. Sent {} bytes while the maximum is {}",
                    msg.len(),
                    sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH
                ),
            };
        }
        let dummy_surb = [0; 16];
        let input_msg = InputMessage(Destination::new(recipient_address, dummy_surb), msg);
        input_tx.send(input_msg).await.unwrap();
        ServerResponse::Send
    }

    async fn handle_fetch(
        mut msg_query: mpsc::UnboundedSender<ReceivedBufferResponse>,
    ) -> ServerResponse {
        trace!("handle_fetch called");
        let (res_tx, res_rx) = oneshot::channel();
        if msg_query.send(res_tx).await.is_err() {
            let e = "Nym-client TCP socket failed to receive messages".to_string();
            error!("{}", e);
            return ServerResponse::Error { message: e };
        }

        let messages = match res_rx.map(|msg| msg).await {
            Ok(messages) => messages,
            Err(e) => {
                warn!("Failed to fetch client messages - {:?}", e);
                return ServerResponse::Error {
                    message: "Server failed to receive messages".to_string(),
                };
            }
        };

        trace!("fetched {} messages", messages.len());
        ServerResponse::Fetch { messages }
    }

    async fn handle_get_clients<T: NymTopology>(
        mut topology_accessor: TopologyAccessor<T>,
    ) -> ServerResponse {
        match topology_accessor.get_current_topology_clone().await {
            Some(topology) => {
                let clients = topology
                    .providers()
                    .iter()
                    .flat_map(|provider| provider.registered_clients.iter())
                    .filter_map(|client| bs58::decode(&client.pub_key).into_vec().ok())
                    .collect();
                ServerResponse::GetClients { clients }
            }
            None => ServerResponse::Error {
                message: "Invalid network topology".to_string(),
            },
        }
    }

    async fn handle_own_details(self_address_bytes: DestinationAddressBytes) -> ServerResponse {
        ServerResponse::OwnDetails {
            address: self_address_bytes.to_bytes().to_vec(),
        }
    }
}

enum ServerResponse {
    Send,
    Fetch { messages: Vec<Vec<u8>> },
    GetClients { clients: Vec<Vec<u8>> },
    OwnDetails { address: Vec<u8> },
    Error { message: String },
}

impl Into<Vec<u8>> for ServerResponse {
    fn into(self) -> Vec<u8> {
        match self {
            ServerResponse::Send => b"ok".to_vec(),
            ServerResponse::Fetch { messages } => encode_fetched_messages(messages),
            ServerResponse::GetClients { clients } => encode_list_of_clients(clients),
            ServerResponse::OwnDetails { address } => address,
            ServerResponse::Error { message } => message.as_bytes().to_vec(),
        }
    }
}

// num_msgs || len1 || len2 || ... || msg1 || msg2 || ...
fn encode_fetched_messages(messages: Vec<Vec<u8>>) -> Vec<u8> {
    // for reciprocal of this look into sfw-provider-requests::responses::PullResponse::from_bytes()

    let num_msgs = messages.len() as u16;
    let msgs_lens: Vec<u16> = messages.iter().map(|msg| msg.len() as u16).collect();

    num_msgs
        .to_be_bytes()
        .to_vec()
        .into_iter()
        .chain(
            msgs_lens
                .into_iter()
                .flat_map(|len| len.to_be_bytes().to_vec().into_iter()),
        )
        .chain(messages.iter().flat_map(|msg| msg.clone().into_iter()))
        .collect()
}

fn encode_list_of_clients(clients: Vec<Vec<u8>>) -> Vec<u8> {
    debug!("client: {:?}", clients);
    // we can just concat all client since all of them got to be 32 bytes long
    // (if not, then we have bigger problem somewhere up the line)

    // converts [[1,2,3],[4,5,6],...] into [1,2,3,4,5,6,...]
    clients.into_iter().flatten().collect()
}

impl ServerResponse {
    fn new_error(message: String) -> ServerResponse {
        ServerResponse::Error { message }
    }
}

async fn handle_connection<T: NymTopology>(
    data: &[u8],
    request_handling_data: RequestHandlingData<T>,
) -> Result<ServerResponse, TCPSocketError> {
    let request = ClientRequest::try_from(data)?;
    let response = match request {
        ClientRequest::Send {
            message,
            recipient_address,
        } => {
            ClientRequest::handle_send(message, recipient_address, request_handling_data.msg_input)
                .await
        }
        ClientRequest::Fetch => ClientRequest::handle_fetch(request_handling_data.msg_query).await,
        ClientRequest::GetClients => {
            ClientRequest::handle_get_clients(request_handling_data.topology_accessor).await
        }
        ClientRequest::OwnDetails => {
            ClientRequest::handle_own_details(request_handling_data.self_address).await
        }
    };

    Ok(response)
}

struct RequestHandlingData<T: NymTopology> {
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<ReceivedBufferResponse>,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
}

async fn accept_connection<T: 'static + NymTopology>(
    mut socket: tokio::net::TcpStream,
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<ReceivedBufferResponse>,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
) {
    let address = socket
        .peer_addr()
        .expect("connected streams should have a peer address");
    debug!("Peer address: {}", address);

    let mut buf = [0u8; 2048];

    // In a loop, read data from the socket and write the data back.
    loop {
        // TODO: shutdowns?

        let response = match socket.read(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                trace!("Remote connection closed.");
                return;
            }
            Ok(n) => {
                let request_handling_data = RequestHandlingData {
                    topology_accessor: topology_accessor.clone(),
                    msg_input: msg_input.clone(),
                    msg_query: msg_query.clone(),
                    self_address: self_address.clone(),
                };
                match handle_connection(&buf[..n], request_handling_data).await {
                    Ok(res) => res,
                    Err(e) => ServerResponse::new_error(format!("{:?}", e)),
                }
            }
            Err(e) => {
                warn!("failed to read from socket; err = {:?}", e);
                return;
            }
        };

        let response_vec: Vec<u8> = response.into();
        if let Err(e) = socket.write_all(&response_vec).await {
            warn!("failed to write reply to socket; err = {:?}", e);
            return;
        }
    }
}

pub(crate) async fn run_tcpsocket<T: 'static + NymTopology>(
    listening_port: u16,
    message_tx: mpsc::UnboundedSender<InputMessage>,
    received_messages_query_tx: mpsc::UnboundedSender<ReceivedBufferResponse>,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
) {
    let address = SocketAddr::new("127.0.0.1".parse().unwrap(), listening_port);
    info!("Starting tcp socket listener at {}", address.to_string());
    let mut listener = tokio::net::TcpListener::bind(address).await.unwrap();

    while let Ok((stream, _)) = listener.accept().await {
        // it's fine to be cloning the channel on all new connection, because in principle
        // this server should only EVER have a single client connected
        tokio::spawn(accept_connection(
            stream,
            message_tx.clone(),
            received_messages_query_tx.clone(),
            self_address.clone(),
            topology_accessor.clone(),
        ));
    }
}

pub(crate) fn start_tcpsocket<T: 'static + NymTopology>(
    handle: &Handle,
    listening_port: u16,
    message_tx: mpsc::UnboundedSender<InputMessage>,
    received_messages_query_tx: mpsc::UnboundedSender<ReceivedBufferResponse>,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
) -> JoinHandle<()> {
    handle.spawn(async move {
        run_tcpsocket(
            listening_port,
            message_tx,
            received_messages_query_tx,
            self_address,
            topology_accessor,
        )
        .await;
    })
}
