use crate::client::received_buffer::BufferResponse;
use crate::client::topology_control::TopologyInnerRef;
use crate::client::InputMessage;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::{mpsc, oneshot};
use futures::future::FutureExt;
use futures::io::Error;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use sphinx::route::{Destination, DestinationAddressBytes};
use std::convert::TryFrom;
use std::io;
use std::net::SocketAddr;
use topology::NymTopology;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

struct Connection<T: NymTopology> {
    address: SocketAddr,
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<BufferResponse>,
    rx: UnboundedReceiver<Message>,
    self_address: DestinationAddressBytes,
    topology: TopologyInnerRef<T>,
    tx: UnboundedSender<Message>,
}

impl<T: NymTopology> Connection<T> {
    async fn handle_text_message(&self, msg: String) -> ServerResponse {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg.clone());

        let request = match ClientRequest::try_from(msg) {
            Ok(req) => req,
            Err(err) => {
                return ServerResponse::Error {
                    // we failed to parse the request
                    message: format!("received invalid request. err: {:?}", err),
                };
            }
        };

        match request {
            ClientRequest::Send {
                message,
                recipient_address,
            } => {
                ClientRequest::handle_send(message, recipient_address, self.msg_input.clone()).await
            }
            ClientRequest::Fetch => ClientRequest::handle_fetch(self.msg_query.clone()).await,
            ClientRequest::GetClients => ClientRequest::handle_get_clients(&self.topology).await,
            ClientRequest::OwnDetails => ClientRequest::handle_own_details(self.self_address).await,
        }
    }

    // Currently our websocket cannot handle binary data, so just close the connection
    // with unsupported close code.
    async fn handle_binary_message(&self, _msg: Vec<u8>) -> Message {
        debug!("Handling binary message request");

        Message::Close(Some(CloseFrame {
            code: CloseCode::Unsupported,
            reason: "binary messages aren't yet supported".into(),
        }))
    }

    // As per RFC6455 5.5.2. and 5.5.3.:
    // Upon receipt of a Ping frame, an endpoint MUST send a Pong frame in response.
    // A Pong frame sent in response to a Ping frame must have identical
    // "Application data" as found in the message body of the Ping frame
    // being replied to.
    async fn handle_ping_message(&self, msg: Vec<u8>) -> Message {
        debug!("Handling binary ping request");

        // As per RFC6455 5.5:
        // All control frames MUST have a payload length of 125 bytes or less
        if msg.len() > 125 {
            return Message::Close(Some(CloseFrame {
                code: CloseCode::Protocol,
                reason: format!("ping message of length {} sent", msg.len()).into(),
            }));
        }

        Message::Pong(msg)
    }

    // As per RFC6455 5.5.3.:
    // A Pong frame MAY be sent unsolicited.  This serves as a
    // unidirectional heartbeat.  A response to an unsolicited Pong frame is
    // not expected.
    // Realistically this handler should never be used,
    // but since we're nice we will reply with a Pong containing original content
    async fn handle_pong_message(&self, msg: Vec<u8>) -> Message {
        debug!("Handling pong message request");

        // As per RFC6455 5.5:
        // All control frames MUST have a payload length of 125 bytes or less
        if msg.len() > 125 {
            return Message::Close(Some(CloseFrame {
                code: CloseCode::Protocol,
                reason: format!("ping message of length {} sent", msg.len()).into(),
            }));
        }

        Message::Pong(msg)
    }

    // As per RFC6455 5.5.1.:
    // If an endpoint receives a Close frame and did not previously send a
    // Close frame, the endpoint MUST send a Close frame in response. (When
    // sending a Close frame in response, the endpoint typically echos the
    // status code it received.)
    async fn handle_close_message(&self, close_frame: Option<CloseFrame<'static>>) -> Message {
        debug!("Handling close message request");

        Message::Close(close_frame)
    }

    async fn handle(mut self) {
        while let Some(msg) = self.rx.next().await {
            trace!("Received a message from {}: {}", self.address, msg);
            let response_message = match msg {
                Message::Text(text_message) => self.handle_text_message(text_message).await.into(),
                Message::Binary(binary_message) => self.handle_binary_message(binary_message).await,
                Message::Ping(ping_message) => self.handle_ping_message(ping_message).await,
                Message::Pong(pong_message) => self.handle_pong_message(pong_message).await,
                Message::Close(close_frame) => self.handle_close_message(close_frame).await,
            };

            if let Err(err) = self.tx.unbounded_send(response_message) {
                error!(
                    "Failed to send response message to accepted connections handler: {}.\n\
                     Shutting off the connection handler",
                    err
                );
                return;
            }
        }
    }
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

impl TryFrom<String> for ClientRequest {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl ClientRequest {
    async fn handle_send(
        msg: String,
        recipient_address: String,
        mut input_tx: mpsc::UnboundedSender<InputMessage>,
    ) -> ServerResponse {
        let message_bytes = msg.into_bytes();
        // TODO: wait until 0.4.0 release to replace those constants with newly exposed
        // sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH
        // we can't do it now for compatibility reasons as most recent sphinx revision
        // has breaking changes due to packet format changes
        let maximum_plaintext_length = sphinx::constants::PAYLOAD_SIZE
            - sphinx::constants::SECURITY_PARAMETER
            - sphinx::constants::DESTINATION_ADDRESS_LENGTH
            - 1;
        if message_bytes.len() > maximum_plaintext_length {
            return ServerResponse::Error {
                message: format!(
                    "too long message. Sent {} bytes while the maximum is {}",
                    message_bytes.len(),
                    maximum_plaintext_length
                )
                .to_string(),
            };
        }

        let address_vec = match base64::decode_config(&recipient_address, base64::URL_SAFE) {
            Err(e) => {
                return ServerResponse::Error {
                    message: e.to_string(),
                };
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

        let input_msg = InputMessage(Destination::new(address, dummy_surb), message_bytes);
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

    async fn handle_get_clients<T: NymTopology>(topology: &TopologyInnerRef<T>) -> ServerResponse {
        let topology_data = &topology.read().await.topology;
        match topology_data {
            Some(topology) => {
                let clients = topology
                    .get_mix_provider_nodes()
                    .iter()
                    .flat_map(|provider| provider.registered_clients.iter())
                    .map(|client| client.pub_key.clone())
                    .collect();
                ServerResponse::GetClients { clients }
            }
            None => ServerResponse::Error {
                message: "Invalid network topology".to_string(),
            },
        }
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
        // it should be safe to call `unwrap` here as the message is generated by the server
        // so if it fails (and consequently panics) it's a bug that should be resolved
        let str_res = serde_json::to_string(&self).unwrap();
        Message::Text(str_res)
    }
}

async fn accept_connection<T: 'static + NymTopology>(
    stream: tokio::net::TcpStream,
    msg_input: mpsc::UnboundedSender<InputMessage>,
    msg_query: mpsc::UnboundedSender<BufferResponse>,
    self_address: DestinationAddressBytes,
    topology: TopologyInnerRef<T>,
) {
    let address = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    debug!("Peer address: {}", address);

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

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

    // TODO: make sure this actually doesn't leak memory...
    tokio::spawn(conn.handle());

    while let Some(message) = ws_stream.next().await {
        let message = match message {
            Ok(msg) => msg,
            Err(err) => {
                error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                return;
            }
        };

        let mut should_close = false;
        if message.is_close() {
            should_close = true;
        }

        if let Err(err) = msg_tx.unbounded_send(message) {
            error!(
                "Failed to forward request. Closing the socket connection: {}",
                err
            );
            return;
        }
        // if the received message is a close message it means we will reply with a close
        // or it is a reply to our close, either way as per RFC6455 5.5.1 we should close the
        // underlying TCP connection:
        // After both sending and receiving a Close message, an endpoint
        // considers the WebSocket connection closed and MUST close the
        // underlying TCP connection. The server MUST close the underlying TCP
        // connection immediately;

        if let Some(resp) = response_rx.next().await {
            if let Err(err) = ws_stream.send(resp).await {
                warn!(
                    "Failed to send message over websocket: {}. Assuming the connection is dead.",
                    err
                );
                return;
            }
        }

        if should_close {
            info!("Closing the websocket connection");
            return;
        }
    }
}

pub async fn start_websocket<T: 'static + NymTopology>(
    address: SocketAddr,
    message_tx: mpsc::UnboundedSender<InputMessage>,
    received_messages_query_tx: mpsc::UnboundedSender<BufferResponse>,
    self_address: DestinationAddressBytes,
    topology: TopologyInnerRef<T>,
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
