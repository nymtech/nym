// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::allowed_hosts::{HostsStore, OutboundRequestFilter};
//use crate::closed_connection_announcer::ClosedConnectionAnnouncer;
use crate::connection::Connection;
use crate::error::NetworkRequesterError;
use crate::statistics::ServiceStatisticsCollector;
use crate::websocket;
use crate::websocket::TSWebsocketStream;
use futures::channel::mpsc;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::receiver::ReconstructedMessage;
use proxy_helpers::connection_controller::{
    ClosedConnectionReceiver, Controller, ControllerCommand, ControllerSender,
};
use socks5_requests::{
    ConnectionId, Message as Socks5Message, NetworkRequesterResponse, Request, Response,
};
use statistics_common::collector::StatisticsSender;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use task::ShutdownListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

// Since it's an atomic, it's safe to be kept static and shared across threads
static ACTIVE_PROXIES: AtomicUsize = AtomicUsize::new(0);

pub struct ServiceProvider {
    listening_address: String,
    outbound_request_filter: OutboundRequestFilter,
    open_proxy: bool,
    enable_statistics: bool,
    stats_provider_addr: Option<Recipient>,
}

impl ServiceProvider {
    pub fn new(
        listening_address: String,
        open_proxy: bool,
        enable_statistics: bool,
        stats_provider_addr: Option<Recipient>,
    ) -> ServiceProvider {
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
            open_proxy,
            enable_statistics,
            stats_provider_addr,
        }
    }

    /// Listens for any messages from `mix_reader` that should be written back to the mix network
    /// via the `websocket_writer`.
    async fn mixnet_response_listener(
        mut websocket_writer: SplitSink<TSWebsocketStream, Message>,
        mut mix_reader: mpsc::UnboundedReceiver<(Socks5Message, Recipient)>,
        stats_collector: Option<ServiceStatisticsCollector>,
        mut closed_connection_rx: ClosedConnectionReceiver,
    ) {
        loop {
            tokio::select! {
                // TODO: wire SURBs in here once they're available
                socks5_msg = mix_reader.next() => {
                    if let Some((msg, return_address)) = socks5_msg {
                        if let Some(stats_collector) = stats_collector.as_ref() {
                            if let Some(remote_addr) = stats_collector
                                .connected_services
                                .read()
                                .await
                                .get(&msg.conn_id())
                            {
                                stats_collector
                                    .response_stats_data
                                    .write()
                                    .await
                                    .processed(remote_addr, msg.size() as u32);
                            }
                        }
                        let conn_id = msg.conn_id();

                        // make 'request' to native-websocket client
                        let response_message = ClientRequest::Send {
                            recipient: return_address,
                            message: msg.into_bytes(),
                            with_reply_surb: false,
                            connection_id: conn_id,
                        };

                        let message = Message::Binary(response_message.serialize());
                        websocket_writer.send(message).await.unwrap();
                    } else {
                        log::error!("Exiting: channel closed!");
                        break;
                    }
                },
                Some(id) = closed_connection_rx.next() => {
                    let msg = ClientRequest::ClosedConnection(id);
                    let ws_msg = Message::Binary(msg.serialize());
                    websocket_writer.send(ws_msg).await.unwrap();
                }
            }
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
        mix_input_sender: mpsc::UnboundedSender<(Socks5Message, Recipient)>,
        shutdown: ShutdownListener,
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
                    .unbounded_send((
                        Socks5Message::Response(Response::new(conn_id, Vec::new(), true)),
                        return_address,
                    ))
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
        conn.run_proxy(mix_receiver, mix_input_sender, shutdown)
            .await;

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
        mix_input_sender: &mpsc::UnboundedSender<(Socks5Message, Recipient)>,
        conn_id: ConnectionId,
        remote_addr: String,
        return_address: Recipient,
        shutdown: ShutdownListener,
    ) {
        if !self.open_proxy && !self.outbound_request_filter.check(&remote_addr) {
            let log_msg = format!("Domain {:?} failed filter check", remote_addr);
            log::info!("{}", log_msg);
            mix_input_sender
                .unbounded_send((
                    Socks5Message::NetworkRequesterResponse(NetworkRequesterResponse::new(
                        conn_id, log_msg,
                    )),
                    return_address,
                ))
                .unwrap();
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
                shutdown,
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
        raw_request: &[u8],
        controller_sender: &mut ControllerSender,
        mix_input_sender: &mpsc::UnboundedSender<(Socks5Message, Recipient)>,
        stats_collector: Option<ServiceStatisticsCollector>,
        shutdown: ShutdownListener,
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
                    if let Some(stats_collector) = stats_collector {
                        stats_collector
                            .connected_services
                            .write()
                            .await
                            .insert(req.conn_id, req.remote_addr.clone());
                    }
                    self.handle_proxy_connect(
                        controller_sender,
                        mix_input_sender,
                        req.conn_id,
                        req.remote_addr,
                        req.return_address,
                        shutdown,
                    )
                }

                Request::Send(conn_id, data, closed) => {
                    if let Some(stats_collector) = stats_collector {
                        if let Some(remote_addr) = stats_collector
                            .connected_services
                            .read()
                            .await
                            .get(&conn_id)
                        {
                            stats_collector
                                .request_stats_data
                                .write()
                                .await
                                .processed(remote_addr, data.len() as u32);
                        }
                    }
                    self.handle_proxy_send(controller_sender, conn_id, data, closed)
                }
            },
            Socks5Message::Response(_) | Socks5Message::NetworkRequesterResponse(_) => {}
        }
    }

    /// Start all subsystems
    pub async fn run(&mut self) -> Result<(), NetworkRequesterError> {
        let websocket_stream = self.connect_websocket(&self.listening_address).await?;

        // split the websocket so that we could read and write from separate threads
        let (websocket_writer, mut websocket_reader) = websocket_stream.split();

        // channels responsible for managing messages that are to be sent to the mix network. The receiver is
        // going to be used by `mixnet_response_listener`
        let (mix_input_sender, mix_input_receiver) =
            mpsc::unbounded::<(Socks5Message, Recipient)>();

        // Used to notify tasks to shutdown
        let shutdown = task::ShutdownNotifier::default();

        // Channel for announcing closed connections by the controller
        let (closed_connection_tx, closed_connection_rx) = mpsc::unbounded();

        // Controller for managing all active connections.
        // We provide it with a ShutdownListener since it requires it, even though for the network
        // requester shutdown signalling is not yet fully implemented.
        let (mut active_connections_controller, mut controller_sender) =
            Controller::new(shutdown.subscribe(), closed_connection_tx);

        tokio::spawn(async move {
            active_connections_controller.run().await;
        });

        let stats_collector = if self.enable_statistics {
            let stats_collector =
                ServiceStatisticsCollector::new(self.stats_provider_addr, mix_input_sender.clone())
                    .await
                    .expect("Service statistics collector could not be bootstrapped");
            let mut stats_sender = StatisticsSender::new(stats_collector.clone());

            tokio::spawn(async move {
                stats_sender.run().await;
            });
            Some(stats_collector)
        } else {
            None
        };

        let stats_collector_clone = stats_collector.clone();
        // start the listener for mix messages
        tokio::spawn(async move {
            Self::mixnet_response_listener(
                websocket_writer,
                mix_input_receiver,
                stats_collector_clone,
                closed_connection_rx,
            )
            .await;
        });

        println!("\nAll systems go. Press CTRL-C to stop the server.");
        // for each incoming message from the websocket... (which in 99.99% cases is going to be a mix message)
        loop {
            let received = match Self::read_websocket_message(&mut websocket_reader).await {
                Some(msg) => msg,
                None => {
                    error!("The websocket stream has finished!");
                    return Ok(());
                }
            };

            let raw_message = received.message;
            // TODO: here be potential SURB (i.e. received.reply_SURB)

            self.handle_proxy_message(
                &raw_message,
                &mut controller_sender,
                &mix_input_sender,
                stats_collector.clone(),
                shutdown.subscribe(),
            )
            .await;
        }
    }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(
        &self,
        uri: &str,
    ) -> Result<TSWebsocketStream, NetworkRequesterError> {
        match websocket::Connection::new(uri).connect().await {
            Ok(ws_stream) => {
                info!("* connected to local websocket server at {}", uri);
                Ok(ws_stream)
            }
            Err(err) => {
                log::error!(
                    "Error: websocket connection attempt failed, is the Nym client running?"
                );
                Err(err.into())
            }
        }
    }
}
