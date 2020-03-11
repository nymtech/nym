use crate::client::received_buffer::ReceivedBufferRequestSender;
use crate::client::topology_control::TopologyAccessor;
use crate::client::{InputMessage, InputMessageSender};
use crate::sockets::websocket::types::{ClientRequest, ServerResponse};
use futures::channel::oneshot;
use futures::{SinkExt, StreamExt};
use log::*;
use sphinx::route::{Destination, DestinationAddressBytes};
use std::convert::TryFrom;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message};
use tokio_tungstenite::WebSocketStream;
use topology::NymTopology;

#[derive(Clone)]
pub(crate) struct ConnectionData<T: NymTopology> {
    msg_input: InputMessageSender,
    msg_query: ReceivedBufferRequestSender,
    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
}

impl<T: NymTopology> ConnectionData<T> {
    pub(crate) fn new(
        msg_input: InputMessageSender,
        msg_query: ReceivedBufferRequestSender,
        self_address: DestinationAddressBytes,
        topology_accessor: TopologyAccessor<T>,
    ) -> Self {
        ConnectionData {
            msg_input,
            msg_query,
            self_address,
            topology_accessor,
        }
    }
}

pub(crate) struct Connection<T: NymTopology> {
    ws_stream: WebSocketStream<tokio::net::TcpStream>,

    msg_input: InputMessageSender,
    msg_query: ReceivedBufferRequestSender,

    self_address: DestinationAddressBytes,
    topology_accessor: TopologyAccessor<T>,
}

impl<T: NymTopology> Connection<T> {
    pub(crate) async fn try_accept(
        raw_stream: tokio::net::TcpStream,
        connection_data: ConnectionData<T>,
    ) -> Option<Self> {
        // perform ws handshake
        let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
            Ok(ws_stream) => ws_stream,
            Err(e) => {
                error!("Error during the websocket handshake occurred - {}", e);
                return None;
            }
        };

        Some(Connection {
            ws_stream,
            msg_input: connection_data.msg_input,
            msg_query: connection_data.msg_query,
            self_address: connection_data.self_address,
            topology_accessor: connection_data.topology_accessor,
        })
    }

    fn handle_text_send(&self, msg: String, recipient_address: String) -> ServerResponse {
        let message_bytes = msg.into_bytes();
        if message_bytes.len() > sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH {
            return ServerResponse::Error {
                message: format!(
                    "message too long. Sent {} bytes, but the maximum is {}",
                    message_bytes.len(),
                    sphinx::constants::MAXIMUM_PLAINTEXT_LENGTH
                ),
            };
        }

        // TODO: the below can panic if recipient_address is malformed, but it should be
        // resolved when refactoring sphinx code to make `from_base58_string` return a Result
        let address = DestinationAddressBytes::from_base58_string(recipient_address);
        let dummy_surb = [0; 16];

        let input_msg = InputMessage(Destination::new(address, dummy_surb), message_bytes);
        self.msg_input.unbounded_send(input_msg).unwrap();

        ServerResponse::Send
    }

    async fn handle_text_fetch(&self) -> ServerResponse {
        // send the request to the buffer controller
        let (res_tx, res_rx) = oneshot::channel();
        self.msg_query.unbounded_send(res_tx).unwrap();
        let messages_bytes = res_rx.await.unwrap();

        let messages = messages_bytes
            .into_iter()
            .map(|message| {
                std::str::from_utf8(&message)
                    .unwrap_or_else(|_| {
                        error!("Invalid UTF-8 sequence in response message");
                        ""
                    })
                    .into()
            })
            .collect();

        ServerResponse::Fetch { messages }
    }

    async fn handle_text_get_clients(&mut self) -> ServerResponse {
        match self.topology_accessor.get_all_clients().await {
            Some(clients) => {
                let client_keys = clients.into_iter().map(|client| client.pub_key).collect();
                ServerResponse::GetClients {
                    clients: client_keys,
                }
            }
            None => ServerResponse::Error {
                message: "Invalid network topology".to_string(),
            },
        }
    }

    fn handle_text_own_details(&self) -> ServerResponse {
        ServerResponse::OwnDetails {
            address: self.self_address.to_base58_string(),
        }
    }

    async fn handle_text_message(&mut self, msg: String) -> Message {
        debug!("Handling text message request");
        trace!("Content: {:?}", msg.clone());

        match ClientRequest::try_from(msg) {
            Err(e) => ServerResponse::Error {
                message: format!("received invalid request. err: {:?}", e),
            }
            .into(),
            Ok(req) => match req {
                ClientRequest::Send {
                    message,
                    recipient_address,
                } => self.handle_text_send(message, recipient_address),
                ClientRequest::Fetch => self.handle_text_fetch().await,
                ClientRequest::GetClients => self.handle_text_get_clients().await,
                ClientRequest::OwnDetails => self.handle_text_own_details(),
            }
            .into(),
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

    async fn handle_request(&mut self, raw_request: Message) -> Message {
        match raw_request {
            Message::Text(text_message) => self.handle_text_message(text_message).await,
            Message::Binary(binary_message) => self.handle_binary_message(binary_message).await,
            Message::Ping(ping_message) => self.handle_ping_message(ping_message).await,
            Message::Pong(pong_message) => self.handle_pong_message(pong_message).await,
            Message::Close(close_frame) => self.handle_close_message(close_frame).await,
        }
    }

    pub(crate) async fn start_handling(&mut self) {
        while let Some(raw_message) = self.ws_stream.next().await {
            let raw_message = match raw_message {
                Ok(msg) => msg,
                Err(err) => {
                    error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                    return;
                }
            };

            let response = self.handle_request(raw_message).await;
            let is_close = response.is_close();
            if let Err(err) = self.ws_stream.send(response).await {
                warn!(
                    "Failed to send message over websocket: {}. Assuming the connection is dead.",
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
            if is_close {
                info!("Closing the websocket connection");
                return;
            }
        }
    }
}
