use crate::clients::BufferResponse;
use crate::clients::InputMessage;
use directory_client::presence::Topology;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::{mpsc, oneshot};
use futures::future::FutureExt;
use futures::io::Error;
use futures::{SinkExt, StreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use sphinx::route::{Destination, DestinationAddressBytes};
use std::io;
use std::net::SocketAddr;
use tungstenite::protocol::Message;

struct Connection {
    address: SocketAddr,
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<BufferResponse>,
    rx: UnboundedReceiver<Message>,
    self_address: DestinationAddressBytes,
    topology: Topology,
    tx: UnboundedSender<Message>,
}

#[derive(Debug)]
pub enum WebSocketError {
    FailedToStartSocketError,
    UnknownSocketError,
}

impl From<io::Error> for WebSocketError {
    fn from(err: Error) -> Self {
        use WebSocketError::*;
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ClientRequest {
    Send {
        message: String,
        recipient_address: String,
    },
    Fetch,
    GetClients,
    OwnDetails,
}

impl From<Message> for ClientRequest {
    fn from(msg: Message) -> Self {
        let text_msg = match msg {
            Message::Text(msg) => msg,
            Message::Binary(_) => panic!("binary messages are not supported!"),
            Message::Close(_) => panic!("todo: handle close!"),
            _ => panic!("Other types of messages are also unsupported!"),
        };
        serde_json::from_str(&text_msg).expect("unable to deserialize From<Message> json")
    }
}

impl ClientRequest {
    async fn handle_send(
        msg: String,
        recipient_address: String,
        mut input_tx: mpsc::UnboundedSender<InputMessage>,
    ) -> ServerResponse {
        let address_vec = match base64::decode_config(&recipient_address, base64::URL_SAFE) {
            Err(e) => {
                return ServerResponse::Error {
                    message: e.to_string(),
                }
            }
            Ok(hex) => hex,
        };

        if address_vec.len() != 32 {
            return ServerResponse::Error {
                message: "InvalidDestinationLength".to_string(),
            };
        }

        let mut address = [0; 32];
        address.copy_from_slice(&address_vec);

        let dummy_surb = [0; 16];

        let input_msg = InputMessage(Destination::new(address, dummy_surb), msg.into_bytes());

        println!("ALMOST ABOUT TO SOMEDAY SEND {:?}", input_msg);
        input_tx.send(input_msg).await.unwrap();

        ServerResponse::Send
    }

    async fn handle_fetch(mut msg_query: mpsc::UnboundedSender<BufferResponse>) -> ServerResponse {
        let (res_tx, res_rx) = oneshot::channel();
        if msg_query.send(res_tx).await.is_err() {
            warn!("Failed to handle_fetch. msg_query.send() is an error.");
            return ServerResponse::Error {
                message: "Server failed to receive messages".to_string(),
            };
        }

        let messages = res_rx.map(|msg| msg).await;
        if messages.is_err() {
            warn!("Failed to handle_fetch. messages is an error");
            return ServerResponse::Error {
                message: "Server failed to receive messages".to_string(),
            };
        }

        let messages = messages
            .unwrap()
            .into_iter()
            .map(|message| {
                match std::str::from_utf8(&message) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Invalid UTF-8 sequence in response message: {}", e);
                        ""
                    }
                }
                .to_owned()
            })
            .collect();

        ServerResponse::Fetch { messages }
    }

    async fn handle_get_clients(topology: Topology) -> ServerResponse {
        let clients = topology
            .mix_provider_nodes
            .into_iter()
            .flat_map(|provider| provider.registered_clients.into_iter())
            .map(|client| client.pub_key)
            .collect();
        ServerResponse::GetClients { clients }
    }

    async fn handle_own_details(self_address_bytes: DestinationAddressBytes) -> ServerResponse {
        let self_address = base64::encode_config(&self_address_bytes, base64::URL_SAFE);
        ServerResponse::OwnDetails {
            address: self_address,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ServerResponse {
    Send,
    Fetch { messages: Vec<String> },
    GetClients { clients: Vec<String> },
    OwnDetails { address: String },
    Error { message: String },
}

impl Into<Message> for ServerResponse {
    fn into(self) -> Message {
        let str_res = serde_json::to_string(&self).unwrap();
        Message::Text(str_res)
    }
}

async fn handle_connection(conn: Connection) {
    let mut conn = conn;
    while let Some(msg) = conn.rx.next().await {
        println!("Received a message from {}: {}", conn.address, msg);
        let request: ClientRequest = msg.into();

        let response = match request {
            ClientRequest::Send {
                message,
                recipient_address,
            } => {
                ClientRequest::handle_send(message, recipient_address, conn.msg_input.clone()).await
            }
            ClientRequest::Fetch => ClientRequest::handle_fetch(conn.msg_query.clone()).await,
            ClientRequest::GetClients => {
                ClientRequest::handle_get_clients(conn.topology.clone()).await
            }
            ClientRequest::OwnDetails => ClientRequest::handle_own_details(conn.self_address).await,
        };

        conn.tx
            .unbounded_send(response.into())
            .expect("Failed to forward message");
    }
}

async fn accept_connection(
    stream: tokio::net::TcpStream,
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<BufferResponse>,
    self_address: DestinationAddressBytes,
    topology: Topology,
) {
    warn!("accept_connection");
    let address = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    println!("Peer address: {}", address);

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("New WebSocket connection: {}", address);

    // Create a channel for our stream, which other sockets will use to
    // send us messages. Then register our address with the stream to send
    // data to us.
    let (msg_tx, msg_rx) = futures::channel::mpsc::unbounded();
    let (response_tx, mut response_rx) = futures::channel::mpsc::unbounded();
    let conn = Connection {
        address,
        rx: msg_rx,
        tx: response_tx,
        topology,
        msg_input,
        msg_query,
        self_address,
    };
    tokio::spawn(handle_connection(conn));

    while let Some(message) = ws_stream.next().await {
        let message = message.expect("Failed to get request");
        msg_tx
            .unbounded_send(message)
            .expect("Failed to forward request");
        if let Some(resp) = response_rx.next().await {
            ws_stream.send(resp).await.expect("Failed to send response");
        }
    }
}

pub async fn start_websocket(
    address: SocketAddr,
    message_tx: mpsc::UnboundedSender<InputMessage>,
    received_messages_query_tx: mpsc::UnboundedSender<BufferResponse>,
    self_address: DestinationAddressBytes,
    topology: Topology,
) -> Result<(), WebSocketError> {
    let mut listener = tokio::net::TcpListener::bind(address).await?;

    while let Ok((stream, _)) = listener.accept().await {
        // it's fine to be cloning the channel on all new connection, because in principle
        // this server should only EVER have a single client connected
        tokio::spawn(accept_connection(
            stream,
            message_tx.clone(),
            received_messages_query_tx.clone(),
            self_address,
            topology.clone(),
        ));
    }

    eprintln!("The websocket went kaput...");
    Ok(())
}
