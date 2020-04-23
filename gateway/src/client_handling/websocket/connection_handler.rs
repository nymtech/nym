use crate::client_handling::clients_handler::{ClientsHandlerRequest, ClientsHandlerRequestSender};
use crate::client_handling::websocket::message_receiver::{MixMessageReceiver, MixMessageSender};
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use gateway_requests::auth_token::AuthToken;
use gateway_requests::types::{ClientRequest, ServerResponse};
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::{prelude::*, stream::StreamExt, sync::Notify};
use tokio_tungstenite::{
    tungstenite::{protocol::Message, Error as WsError},
    WebSocketStream,
};

// EXPERIMENT:
struct MixMessagesHandle {
    sender: MixMessageSender,
    receiver: MixMessageReceiver,

    shutdown: Arc<Notify>,
}

impl MixMessagesHandle {
    fn new(shutdown: Notify) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        MixMessagesHandle {
            sender,
            receiver,
            shutdown: Arc::new(Notify::new()),
        }
    }

    fn shutdown(&self) {
        self.shutdown.notify()
    }

    fn get_sender(&self) -> MixMessageSender {
        self.sender.clone()
    }

    fn start_accepting(&self) {
        let shutdown_signal = Arc::clone(&self.shutdown);

        // note to the graceful pull request reviewer: this is by no means how we'd be handling
        // proper shutdown signals, this is more of an experiment that happened to do exactly
        // what I needed in here (basically to not leak memory)
        tokio::spawn(async move {
            tokio::select! {
                            // TODO: solve borrow issue and figure out how to push result to socket
            //                msg = self.receiver.next() => {
            //
            //                }

                            _ = shutdown_signal.notified() => {
                                info!("received shutdown notification")
                            }
                        }
        });
    }
}

// TODO: note for my future self to consider the following idea:
// split the socket connection into sink and stream
// stream will be for reading explicit requests
// and sink for pumping responses AND mix traffic
// but as byproduct this might (or might not) break the clean "SocketStream" enum here

enum SocketStream<S: AsyncRead + AsyncWrite + Unpin> {
    RawTCP(S),
    UpgradedWebSocket(WebSocketStream<S>),
    Invalid,
}

pub(crate) struct Handle<S: AsyncRead + AsyncWrite + Unpin> {
    address: Option<DestinationAddressBytes>,
    authenticated: bool,
    clients_handler_sender: ClientsHandlerRequestSender,
    socket_connection: SocketStream<S>,
}

impl<S> Handle<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(conn: S, clients_handler_sender: ClientsHandlerRequestSender) -> Self {
        Handle {
            address: None,
            authenticated: false,
            clients_handler_sender,
            socket_connection: SocketStream::RawTCP(conn),
        }
    }

    async fn perform_websocket_handshake(&mut self) {
        self.socket_connection =
            match std::mem::replace(&mut self.socket_connection, SocketStream::Invalid) {
                SocketStream::RawTCP(conn) => {
                    // TODO: perhaps in the future, rather than panic here (and uncleanly shut tcp stream)
                    // return a result with an error?
                    let ws_stream = tokio_tungstenite::accept_async(conn)
                        .await
                        .expect("Failed to perform websocket handshake");
                    SocketStream::UpgradedWebSocket(ws_stream)
                }
                other => other,
            }
    }

    async fn next_websocket_request(&mut self) -> Option<Result<Message, WsError>> {
        match self.socket_connection {
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.next().await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    async fn send_websocket_response(&mut self, msg: Message) -> Result<(), WsError> {
        match self.socket_connection {
            // TODO: more closely investigate difference between `Sink::send` and `Sink::send_all`
            // it got something to do with batching and flushing - it might be important if it
            // turns out somehow we've got a bottleneck here
            SocketStream::UpgradedWebSocket(ref mut ws_stream) => ws_stream.send(msg).await,
            _ => panic!("impossible state - websocket handshake was somehow reverted"),
        }
    }

    fn disconnect(&self) {
        // if we never established what is the address of the client, its connection was never
        // announced hence we do not need to send 'disconnect' message
        self.address.as_ref().map(|addr| {
            self.clients_handler_sender
                .unbounded_send(ClientsHandlerRequest::Disconnect(addr.clone()))
                .unwrap();
        });
    }

    async fn handle_binary(&self, bin_msg: Vec<u8>) -> Message {
        trace!("Handling binary message (presumably sphinx packet)");

        // if it's binary, it MUST BE a sphinx packet. We can't look into it, but let's at least
        // validate its size.
        if bin_msg.len() != nymsphinx::PACKET_SIZE {}
        unimplemented!()
    }

    async fn handle_authenticate(&mut self, address: String, token: String) -> ServerResponse {
        // TODO: https://github.com/nymtech/sphinx/issues/57 to resolve possible panics
        // because we do **NOT** trust whatever garbage client just sent.
        let address = DestinationAddressBytes::from_base58_string(address);
        let token = match AuthToken::try_from_base58_string(token) {
            Ok(token) => token,
            Err(e) => {
                trace!("failed to parse received AuthToken: {:?}", e);
                return ServerResponse::new_error("malformed authentication token").into();
            }
        };

        // TODO: how to deal with the mix sender channel?

        //        let (res_sender, res_receiver) = oneshot::channel();
        //        let clients_handler_request =
        //            ClientsHandlerRequest::Authenticate(address, token, res_sender);
        //        self.clients_handler_sender
        //            .unbounded_send(clients_handler_request)
        //            .unwrap(); // the receiver MUST BE alive
        //
        //        let client_sender = match res_receiver.await.unwrap() {
        //            ClientsHandlerResponse::IsOnline(client_sender) => client_sender,
        //            _ => panic!("received response to wrong query!"), // again, this should NEVER happen
        //        };

        unimplemented!()
    }

    async fn handle_register(&mut self, address: String) -> ServerResponse {
        // TODO: https://github.com/nymtech/sphinx/issues/57 to resolve possible panics
        // because we do **NOT** trust whatever garbage client just sent.
        let address = DestinationAddressBytes::from_base58_string(address);

        // TODO: how to deal with the mix sender channel?

        unimplemented!()
    }

    async fn handle_text(&mut self, text_msg: String) -> Message {
        trace!("Handling text message (presumably control message)");

        match ClientRequest::try_from(text_msg) {
            Err(e) => ServerResponse::Error {
                message: format!("received invalid request. err: {:?}", e),
            },
            Ok(req) => match req {
                ClientRequest::Authenticate { address, token } => {
                    self.handle_authenticate(address, token).await
                }
                ClientRequest::Register { address } => self.handle_register(address).await,
            },
        }
        .into()
    }

    async fn handle_request(&mut self, raw_request: Message) -> Option<Message> {
        // apparently tungstenite auto-handles ping/pong/close messages so for now let's ignore
        // them and let's test that claim. If that's not the case, just copy code from
        // desktop nym-client websocket as I've manually handled everything there
        match raw_request {
            Message::Binary(bin_msg) => Some(self.handle_binary(bin_msg).await),
            Message::Text(text_msg) => Some(self.handle_text(text_msg).await),
            _ => None,
        }
    }

    // TODO: FUTURE SELF, START HERE TOMORROW:
    // change into "wait for authentication" and then listen for either client request
    // or mix message channel message!
    async fn listen_for_requests(&mut self) {
        trace!("Started listening for incoming requests...");

        while let Some(msg) = self.next_websocket_request().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(err) => {
                    error!("failed to obtain message from websocket stream! stopping connection handler: {}", err);
                    break;
                }
            };

            if msg.is_close() {
                break;
            }

            if let Some(response) = self.handle_request(msg).await {
                if let Err(err) = self.send_websocket_response(response).await {
                    warn!(
                        "Failed to send message over websocket: {}. Assuming the connection is dead.",
                        err
                    );
                    break;
                }
            }
        }

        self.disconnect();
        trace!("The stream was closed!");
    }

    pub(crate) async fn start_handling(&mut self) {
        self.perform_websocket_handshake().await;
        trace!("Managed to perform websocket handshake!");
        self.listen_for_requests().await;
    }
}
