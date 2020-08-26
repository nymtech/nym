use crate::connection::Connection;
use crate::websocket;
use futures::channel::mpsc;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use simple_socks5_requests::{Request, Response};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use utils::connection_controller::{Controller, ControllerCommand};
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

pub struct ServiceProvider {
    listening_address: String,
}

impl ServiceProvider {
    pub fn new(listening_address: String) -> ServiceProvider {
        ServiceProvider { listening_address }
    }

    /// Listens for any messages from `mix_reader` that should be written back to the mix network
    /// via the `websocket_writer`.
    async fn mixnet_response_listener(
        mut websocket_writer: SplitSink<WebSocketStream<TcpStream>, Message>,
        mut mix_reader: mpsc::UnboundedReceiver<(Response, Recipient)>,
    ) {
        // TODO: wire SURBs in here once they're available
        while let Some((response, return_address)) = mix_reader.next().await {
            // make 'request' to native-websocket client
            let response_message = ClientRequest::Send {
                recipient: return_address,
                message: response.into_bytes(),
                with_reply_surb: false,
            };

            let message = Message::Binary(response_message.serialize());
            websocket_writer.send(message).await.unwrap();
        }
    }

    /// Start all subsystems
    pub async fn run(&mut self) {
        let websocket_stream = self.connect_websocket(&self.listening_address).await;

        // split the websocket so that we could read and write from separate threads
        let (websocket_writer, mut websocket_reader) = websocket_stream.split();

        // channels responsible for managing messages that are to be sent to the mix network. The receiver is
        // going to be used by `mixnet_response_listener`
        let (mix_input_sender, mix_input_receiver) = mpsc::unbounded::<(Response, Recipient)>();

        // controller for managing all active connections
        let (mut active_connections_controller, controller_sender) = Controller::new();
        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        // start the listener for mix messages
        tokio::spawn(async move {
            Self::mixnet_response_listener(websocket_writer, mix_input_receiver).await;
        });

        println!("\nAll systems go. Press CTRL-C to stop the server.");

        // for each incoming message from the websocket... (which in 99.99% cases is going to be a mix message)
        while let Some(msg) = websocket_reader.next().await {
            let data = msg
                .expect("we failed to read from the websocket!")
                .into_data();

            // try to recover the actual message from the mix network...
            let deserialized_message = match ServerResponse::deserialize(&data) {
                Ok(deserialized) => deserialized,
                Err(err) => {
                    error!(
                        "Failed to deserialize received websocket message! - {}",
                        err
                    );
                    continue;
                }
            };

            let received = match deserialized_message {
                ServerResponse::Received(received) => received,
                ServerResponse::Error(err) => {
                    panic!("received error from native client! - {}", err)
                }
                _ => unimplemented!("probably should never be reached?"),
            };

            let raw_message = received.message;
            // TODO: here be potential SURB (i.e. received.reply_SURB)

            // try to treat each received mix message as a service provider request
            let deserialized_request = match Request::try_from_bytes(&raw_message) {
                Ok(request) => request,
                Err(err) => {
                    error!("Failed to deserialized received request! - {}", err);
                    continue;
                }
            };

            match deserialized_request {
                Request::Connect {
                    conn_id,
                    remote_addr,
                    data,
                    return_address,
                } => {
                    // Connect implies it's a fresh connection - register it with our controller
                    let (mix_sender, mix_receiver) = mpsc::unbounded();
                    controller_sender
                        .unbounded_send(ControllerCommand::Insert(conn_id, mix_sender))
                        .unwrap();

                    let controller_sender_clone = controller_sender.clone();
                    // and start the proxy for this connection
                    let mix_input_sender_clone = mix_input_sender.clone();
                    tokio::spawn(async move {
                        let mut conn = match Connection::new(
                            conn_id,
                            remote_addr.clone(),
                            &data,
                            return_address,
                        )
                        .await
                        {
                            Ok(conn) => conn,
                            Err(err) => {
                                error!(
                                    "error while connecting to {:?} ! - {:?}",
                                    remote_addr.clone(),
                                    err
                                );
                                return;
                            }
                        };

                        info!("Starting proxy for {}", remote_addr.clone());
                        conn.run_proxy(mix_receiver, mix_input_sender_clone).await;
                        // proxy is done - remove the access channel from the controller
                        controller_sender_clone
                            .unbounded_send(ControllerCommand::Remove(conn_id))
                            .unwrap();
                        info!("Proxy for {} is finished", remote_addr);
                    });
                }
                // on send just tell the controller to send that data to the correct connection
                Request::Send(conn_id, data, closed) => controller_sender
                    .unbounded_send(ControllerCommand::Send(conn_id, data, closed))
                    .unwrap(),
            }
        }
    }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(&self, uri: &str) -> WebSocketStream<TcpStream> {
        let ws_stream = match websocket::Connection::new(uri).connect().await {
            Ok(ws_stream) => {
                info!("* connected to local websocket server at {}", uri);
                ws_stream
            }
            Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                panic!("Error: websocket connection attempt failed, is the Nym client running?")
            }
        };
        return ws_stream;
    }
}
