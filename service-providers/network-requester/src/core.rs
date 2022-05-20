// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::allowed_hosts::{HostsStore, OutboundRequestFilter};
use crate::connection::Connection;
use crate::statistics::{Statistics, StatsData, Timer};
use crate::websocket;
use crate::websocket::TSWebsocketStream;
use futures::channel::mpsc;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::{ClientIdentity, Recipient};
use nymsphinx::receiver::ReconstructedMessage;
use proxy_helpers::connection_controller::{Controller, ControllerCommand, ControllerSender};
use socks5_requests::{ConnectionId, Message as Socks5Message, Request, Response};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::protocol::Message;
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

// Since it's an atomic, it's safe to be kept static and shared across threads
static ACTIVE_PROXIES: AtomicUsize = AtomicUsize::new(0);

pub struct ServiceProvider {
    listening_address: String,
    description: String,
    #[cfg(feature = "stats-service")]
    db_path: PathBuf,
    outbound_request_filter: OutboundRequestFilter,
    open_proxy: bool,
}

impl ServiceProvider {
    pub fn new(
        listening_address: String,
        description: String,
        open_proxy: bool,
    ) -> ServiceProvider {
        let allowed_hosts = HostsStore::new(
            HostsStore::default_base_dir(),
            PathBuf::from("allowed.list"),
        );

        let unknown_hosts = HostsStore::new(
            HostsStore::default_base_dir(),
            PathBuf::from("unknown.list"),
        );

        #[cfg(feature = "stats-service")]
        let db_path = HostsStore::default_base_dir()
            .join("service-providers")
            .join("network-requester")
            .join("db.sqlite");

        let outbound_request_filter = OutboundRequestFilter::new(allowed_hosts, unknown_hosts);
        ServiceProvider {
            listening_address,
            description,
            #[cfg(feature = "stats-service")]
            db_path,
            outbound_request_filter,
            open_proxy,
        }
    }

    /// Listens for any messages from `mix_reader` that should be written back to the mix network
    /// via the `websocket_writer`.
    async fn mixnet_response_listener(
        mut websocket_writer: SplitSink<TSWebsocketStream, Message>,
        mut mix_reader: mpsc::UnboundedReceiver<(Response, Recipient)>,
        response_stats_data: &Arc<RwLock<StatsData>>,
    ) {
        // TODO: wire SURBs in here once they're available
        while let Some((response, return_address)) = mix_reader.next().await {
            response_stats_data
                .write()
                .await
                .processed(return_address.identity(), response.data.len() as u32);
            // make 'request' to native-websocket client
            let response_message = ClientRequest::Send {
                recipient: return_address,
                message: Socks5Message::Response(response).into_bytes(),
                with_reply_surb: false,
            };

            let message = Message::Binary(response_message.serialize());
            websocket_writer.send(message).await.unwrap();
        }
    }

    async fn read_websocket_message(
        websocket_reader: &mut SplitStream<TSWebsocketStream>,
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
        controller_sender: ControllerSender,
        mix_input_sender: mpsc::UnboundedSender<(Response, Recipient)>,
    ) {
        let mut conn = match Connection::new(conn_id, remote_addr.clone(), return_address).await {
            Ok(conn) => conn,
            Err(err) => {
                error!(
                    "error while connecting to {:?} ! - {:?}",
                    remote_addr.clone(),
                    err
                );

                // inform the remote that the connection is closed before it even was established
                mix_input_sender
                    .unbounded_send((Response::new(conn_id, Vec::new(), true), return_address))
                    .unwrap();

                return;
            }
        };

        // Connect implies it's a fresh connection - register it with our controller
        let (mix_sender, mix_receiver) = mpsc::unbounded();
        controller_sender
            .unbounded_send(ControllerCommand::Insert(conn_id, mix_sender))
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
        return_address: Recipient,
    ) {
        if !self.open_proxy && !self.outbound_request_filter.check(&remote_addr) {
            log::info!("Domain {:?} failed filter check", remote_addr);
            return;
        }

        let controller_sender_clone = controller_sender.clone();
        let mix_input_sender_clone = mix_input_sender.clone();

        // and start the proxy for this connection
        tokio::spawn(async move {
            Self::start_proxy(
                conn_id,
                remote_addr,
                return_address,
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

    async fn handle_proxy_message(
        &mut self,
        #[cfg(feature = "stats-service")] storage: &crate::storage::NetworkRequesterStorage,
        raw_request: &[u8],
        controller_sender: &mut ControllerSender,
        mix_input_sender: &mpsc::UnboundedSender<(Response, Recipient)>,
        request_stats_data: &Arc<RwLock<StatsData>>,
        connected_clients: &mut HashMap<ConnectionId, ClientIdentity>,
    ) {
        let deserialized_msg = match Socks5Message::try_from_bytes(raw_request) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Failed to deserialized received message! - {}", err);
                return;
            }
        };
        match deserialized_msg {
            Socks5Message::Request(deserialized_request) => match deserialized_request {
                Request::Connect(req) => {
                    connected_clients.insert(req.conn_id, *req.return_address.identity());
                    self.handle_proxy_connect(
                        controller_sender,
                        mix_input_sender,
                        req.conn_id,
                        req.remote_addr,
                        req.return_address,
                    )
                }

                Request::Send(conn_id, data, closed) => {
                    if let Some(client_identity) = connected_clients.get(&conn_id) {
                        request_stats_data
                            .write()
                            .await
                            .processed(client_identity, data.len() as u32);
                    }
                    self.handle_proxy_send(controller_sender, conn_id, data, closed)
                }
            },
            Socks5Message::Response(_deserialized_response) =>
            {
                #[cfg(feature = "stats-service")]
                match crate::statistics::StatsMessage::from_bytes(&_deserialized_response.data) {
                    Ok(data) => {
                        if let Err(e) = storage.insert_service_statistics(data).await {
                            error!("Could not store received statistics: {}", e);
                        }
                    }
                    Err(e) => error!("Malformed statistics received: {}", e),
                }
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

        let (mut timer_sender, timer_receiver) = Timer::new();
        let interval = timer_sender.interval();
        tokio::spawn(async move {
            timer_sender.run().await;
        });

        // controller for managing all active connections
        let (mut active_connections_controller, mut controller_sender) = Controller::new();
        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        let mut stats = Statistics::new(self.description.clone(), interval, timer_receiver);
        let request_stats_data = Arc::clone(stats.request_data());
        let response_stats_data = Arc::clone(stats.response_data());
        let mix_input_sender_clone = mix_input_sender.clone();
        tokio::spawn(async move {
            stats.run(&mix_input_sender_clone).await;
        });

        #[cfg(feature = "stats-service")]
        let storage = crate::storage::NetworkRequesterStorage::init(&self.db_path)
            .await
            .expect("Could not create network requester storage");

        #[cfg(feature = "stats-service")]
        tokio::spawn(
            rocket::build()
                .mount(
                    "/v1",
                    rocket::routes![crate::storage::post_mixnet_statistics],
                )
                .manage(storage.clone())
                .ignite()
                .await
                .expect("Could not ignite stats api service")
                .launch(),
        );

        // start the listener for mix messages
        tokio::spawn(async move {
            Self::mixnet_response_listener(
                websocket_writer,
                mix_input_receiver,
                &response_stats_data,
            )
            .await;
        });

        println!("\nAll systems go. Press CTRL-C to stop the server.");
        // for each incoming message from the websocket... (which in 99.99% cases is going to be a mix message)
        let mut connected_clients = HashMap::new();
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

            self.handle_proxy_message(
                #[cfg(feature = "stats-service")]
                &storage,
                &raw_message,
                &mut controller_sender,
                &mix_input_sender,
                &request_stats_data,
                &mut connected_clients,
            )
            .await;
        }
    }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(&self, uri: &str) -> TSWebsocketStream {
        let ws_stream = match websocket::Connection::new(uri).connect().await {
            Ok(ws_stream) => {
                info!("* connected to local websocket server at {}", uri);
                ws_stream
            }
            Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                panic!("Error: websocket connection attempt failed, is the Nym client running?")
            }
        };
        ws_stream
    }
}
