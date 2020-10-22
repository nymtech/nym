use crate::allowed_hosts::{HostsStore, OutboundRequestFilter};
use crate::connection::Connection;
use crate::websocket;
use futures::channel::mpsc;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::receiver::ReconstructedMessage;
use ordered_buffer::{OrderedMessage, OrderedMessageBuffer};
use proxy_helpers::connection_controller::{Controller, ControllerCommand, ControllerSender};
use socks5_requests::{ConnectionId, Request, Response};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

static ACTIVE_PROXIES: AtomicUsize = AtomicUsize::new(0);

pub struct ServiceProvider {
    listening_address: String,
    outbound_request_filter: OutboundRequestFilter,
}

impl ServiceProvider {
    pub fn new(listening_address: String) -> ServiceProvider {
        let allowed_hosts = HostsStore::new(
            HostsStore::default_base_dir(),
            PathBuf::from("allowed.list"),
        );

        let unknown_hosts = HostsStore::new(
            HostsStore::default_base_dir(),
            PathBuf::from("unknown.list"),
        );
        let outbound_request_filter = OutboundRequestFilter::new(allowed_hosts, unknown_hosts);
        ServiceProvider {
            listening_address,
            outbound_request_filter,
        }
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

    async fn read_websocket_message(
        websocket_reader: &mut SplitStream<WebSocketStream<TcpStream>>,
    ) -> Option<ReconstructedMessage> {
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
            return Some(received);
        }
        None
    }

    async fn start_proxy(
        conn_id: ConnectionId,
        remote_addr: String,
        return_address: Recipient,
        mut ordered_buffer: OrderedMessageBuffer,
        controller_sender: ControllerSender,
        mix_input_sender: mpsc::UnboundedSender<(Response, Recipient)>,
    ) {
        let init_data = ordered_buffer
            .read()
            .expect("Received connect request but it wasn't sequence 0!");

        let mut conn =
            match Connection::new(conn_id, remote_addr.clone(), &init_data, return_address).await {
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

        // Connect implies it's a fresh connection - register it with our controller
        let (mix_sender, mix_receiver) = mpsc::unbounded();
        controller_sender
            .unbounded_send(ControllerCommand::Insert(
                conn_id,
                mix_sender,
                ordered_buffer,
            ))
            .unwrap();

        let old_count = ACTIVE_PROXIES.fetch_add(1, Ordering::SeqCst);
        info!(
            "Starting proxy for {} (currently there are {} proxies being handled)",
            remote_addr,
            old_count + 1
        );

        // run the proxy on the connection
        conn.run_proxy(mix_receiver, mix_input_sender).await;

        // proxy is done - remove the access channel from the controller
        controller_sender
            .unbounded_send(ControllerCommand::Remove(conn_id))
            .unwrap();

        let old_count = ACTIVE_PROXIES.fetch_sub(1, Ordering::SeqCst);
        info!(
            "Proxy for {} is finished  (currently there are {} proxies being handled)",
            remote_addr,
            old_count - 1
        );
    }

    fn handle_proxy_connect(
        &mut self,
        controller_sender: &mut ControllerSender,
        mix_input_sender: &mpsc::UnboundedSender<(Response, Recipient)>,
        conn_id: ConnectionId,
        remote_addr: String,
        message: OrderedMessage,
        return_address: Recipient,
    ) {
        if !self.outbound_request_filter.check(&remote_addr) {
            info!("Domain {:?} failed filter check", remote_addr);
            return;
        }

        let mut ordered_buffer = OrderedMessageBuffer::new();
        ordered_buffer.write(message);

        let controller_sender_clone = controller_sender.clone();
        let mix_input_sender_clone = mix_input_sender.clone();

        // and start the proxy for this connection
        tokio::spawn(async move {
            Self::start_proxy(
                conn_id,
                remote_addr,
                return_address,
                ordered_buffer,
                controller_sender_clone,
                mix_input_sender_clone,
            )
            .await
        });
    }

    fn handle_proxy_send(
        &self,
        controller_sender: &mut ControllerSender,
        conn_id: ConnectionId,
        data: Vec<u8>,
        closed: bool,
    ) {
        controller_sender
            .unbounded_send(ControllerCommand::Send(conn_id, data, closed))
            .unwrap()
    }

    fn handle_proxy_request(
        &mut self,
        raw_request: &[u8],
        controller_sender: &mut ControllerSender,
        mix_input_sender: &mpsc::UnboundedSender<(Response, Recipient)>,
    ) {
        // try to treat each received mix message as a service provider request
        let deserialized_request = match Request::try_from_bytes(&raw_request) {
            Ok(request) => request,
            Err(err) => {
                error!("Failed to deserialized received request! - {}", err);
                return;
            }
        };

        match deserialized_request {
            Request::Connect {
                conn_id,
                remote_addr,
                message,
                return_address,
            } => self.handle_proxy_connect(
                controller_sender,
                mix_input_sender,
                conn_id,
                remote_addr,
                message,
                return_address,
            ),
            Request::Send(conn_id, data, closed) => {
                self.handle_proxy_send(controller_sender, conn_id, data, closed)
            }
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
        let (mut active_connections_controller, mut controller_sender) = Controller::new();
        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        // start the listener for mix messages
        tokio::spawn(async move {
            Self::mixnet_response_listener(websocket_writer, mix_input_receiver).await;
        });

        println!("\nAll systems go. Press CTRL-C to stop the server.");

        // for each incoming message from the websocket... (which in 99.99% cases is going to be a mix message)
        loop {
            let received = match Self::read_websocket_message(&mut websocket_reader).await {
                Some(msg) => msg,
                None => {
                    error!("The websocket stream has finished!");
                    return;
                }
            };

            let raw_message = received.message;
            // TODO: here be potential SURB (i.e. received.reply_SURB)

            self.handle_proxy_request(&raw_message, &mut controller_sender, &mix_input_sender)
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
