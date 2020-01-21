use crate::built_info;
use crate::client::mix_traffic::{MixMessage, MixTrafficController};
use crate::sockets::tcp;
use crate::sockets::ws;
use crate::utils;
use directory_client::presence::Topology;
use futures::channel::mpsc;
use futures::join;
use futures::select;
use futures::{SinkExt, StreamExt};
use log::*;
use sfw_provider_requests::AuthToken;
use sphinx::route::{Destination, DestinationAddressBytes};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Runtime;
use topology::NymTopology;
use crate::client::received_buffer::ReceivedMessagesBuffer;

mod mix_traffic;
pub mod received_buffer;

const LOOP_COVER_AVERAGE_DELAY: f64 = 0.5;
// seconds
const MESSAGE_SENDING_AVERAGE_DELAY: f64 = 0.5;
//  seconds;
const FETCH_MESSAGES_DELAY: f64 = 1.0; // seconds;

pub enum SocketType {
    TCP,
    WebSocket,
    None,
}

pub struct NymClient {
    // to be replaced by something else I guess
    address: DestinationAddressBytes,
    pub input_tx: mpsc::UnboundedSender<InputMessage>,
    // to be used by "send" function or socket, etc
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
    socket_listening_address: SocketAddr,
    directory: String,
    auth_token: Option<AuthToken>,
    socket_type: SocketType,
}

#[derive(Debug)]
pub struct InputMessage(pub Destination, pub Vec<u8>);

impl NymClient {
    pub fn new(
        address: DestinationAddressBytes,
        socket_listening_address: SocketAddr,
        directory: String,
        auth_token: Option<AuthToken>,
        socket_type: SocketType,
    ) -> Self {
        let (input_tx, input_rx) = mpsc::unbounded::<InputMessage>();

        NymClient {
            address,
            input_tx,
            input_rx,
            socket_listening_address,
            directory,
            auth_token,
            socket_type,
        }
    }

    async fn start_loop_cover_traffic_stream(
        mut tx: mpsc::UnboundedSender<MixMessage>,
        our_info: Destination,
        topology: Topology,
    ) {
        loop {
            info!("[LOOP COVER TRAFFIC STREAM] - next cover message!");
            let delay = utils::poisson::sample(LOOP_COVER_AVERAGE_DELAY);
            let delay_duration = Duration::from_secs_f64(delay);
            tokio::time::delay_for(delay_duration).await;
            let cover_message =
                utils::sphinx::loop_cover_message(our_info.address, our_info.identifier, &topology);
            tx.send(MixMessage::new(cover_message.0, cover_message.1))
                .await
                .unwrap();
        }
    }

    async fn control_out_queue(
        mut mix_tx: mpsc::UnboundedSender<MixMessage>,
        mut input_rx: mpsc::UnboundedReceiver<InputMessage>,
        our_info: Destination,
        topology: Topology,
    ) {
        loop {
            info!("[OUT QUEUE] here I will be sending real traffic (or loop cover if nothing is available)");
            // TODO: consider replacing select macro with our own proper future definition with polling
            let traffic_message = select! {
                real_message = input_rx.next() => {
                    info!("[OUT QUEUE] - we got a real message!");
                    if real_message.is_none() {
                        error!("Unexpected 'None' real message!");
                        std::process::exit(1);
                    }
                    let real_message = real_message.unwrap();
                    println!("real: {:?}", real_message);
                    utils::sphinx::encapsulate_message(real_message.0, real_message.1, &topology)
                },

                default => {
                    info!("[OUT QUEUE] - no real message - going to send extra loop cover");
                    utils::sphinx::loop_cover_message(our_info.address, our_info.identifier, &topology)
                }
            };

            mix_tx
                .send(MixMessage::new(traffic_message.0, traffic_message.1))
                .await
                .unwrap();

            let delay_duration = Duration::from_secs_f64(MESSAGE_SENDING_AVERAGE_DELAY);
            tokio::time::delay_for(delay_duration).await;
        }
    }

    async fn start_provider_polling(
        provider_client: provider_client::ProviderClient,
        mut poller_tx: mpsc::UnboundedSender<Vec<Vec<u8>>>,
    ) {
        let loop_message = &utils::sphinx::LOOP_COVER_MESSAGE_PAYLOAD.to_vec();
        let dummy_message = &sfw_provider_requests::DUMMY_MESSAGE_CONTENT.to_vec();
        loop {
            let delay_duration = Duration::from_secs_f64(FETCH_MESSAGES_DELAY);
            tokio::time::delay_for(delay_duration).await;
            info!("[FETCH MSG] - Polling provider...");
            let messages = provider_client.retrieve_messages().await.unwrap();
            let good_messages = messages
                .into_iter()
                .filter(|message| message != loop_message && message != dummy_message)
                .collect();
            // if any of those fails, whole application should blow...
            poller_tx.send(good_messages).await.unwrap();
        }
    }

    pub fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let score_threshold = 0.0;
        println!("Starting nym client");
        let mut rt = Runtime::new()?;

        println!("Trying to obtain valid, healthy, topology");
        let full_topology = Topology::new(self.directory.clone());

        // run a healthcheck to determine healthy-ish nodes:
        // this is a temporary solution as the healthcheck will eventually be moved to validators
        let healthcheck_config = healthcheck::config::HealthCheck {
            directory_server: self.directory.clone(),
            // those are literally unrelevant when running single check
            interval: 100000.0,
            resolution_timeout: 5.0,
            num_test_packets: 2,
        };
        let healthcheck = healthcheck::HealthChecker::new(healthcheck_config);
        let healthcheck_result = rt.block_on(healthcheck.do_check());

        let healthcheck_scores = match healthcheck_result {
            Err(err) => {
                error!(
                    "failed to perform healthcheck to determine healthy topology - {:?}",
                    err
                );
                return Err(Box::new(err));
            }
            Ok(scores) => scores,
        };

        let healthy_topology =
            healthcheck_scores.filter_topology_by_score(&full_topology, score_threshold);

        // for time being assume same versioning, i.e. if client is running X.Y.Z,
        // we're expecting mixes, providers and coconodes to also be running X.Y.Z
        let versioned_healthy_topology = healthy_topology.filter_node_versions(
            built_info::PKG_VERSION,
            built_info::PKG_VERSION,
            built_info::PKG_VERSION,
        );

        // make sure you can still send a packet through the network:
        if !versioned_healthy_topology.can_construct_path_through() {
            error!("No valid path exists in the topology");
            // TODO: replace panic with proper return type
            panic!("No valid path exists in the topology");
        }

        // this is temporary and assumes there exists only a single provider.
        let provider_client_listener_address: SocketAddr = versioned_healthy_topology
            .get_mix_provider_nodes()
            .first()
            .expect("Could not get a provider from the supplied network topology, are you using the right directory server?")
            .client_listener;

        let mut provider_client = provider_client::ProviderClient::new(
            provider_client_listener_address,
            self.address,
            self.auth_token,
        );

        // registration
        rt.block_on(async {
            match self.auth_token {
                None => {
                    let auth_token = provider_client.register().await.unwrap();
                    provider_client.update_token(auth_token);
                    info!("Obtained new token! - {:?}", auth_token);
                }
                Some(token) => println!("Already got the token! - {:?}", token),
            }
        });

        // channels for intercomponent communication
        let (mix_tx, mix_rx) = mpsc::unbounded();
        let (poller_input_tx, poller_input_rx) = mpsc::unbounded();
        let (received_messages_buffer_output_tx, received_messages_buffer_output_rx) =
            mpsc::unbounded();

        let received_messages_buffer = ReceivedMessagesBuffer::new().add_arc_futures_mutex();

        let received_messages_buffer_input_controller_future =
            rt.spawn(ReceivedMessagesBuffer::run_poller_input_controller(
                received_messages_buffer.clone(),
                poller_input_rx,
            ));
        let received_messages_buffer_output_controller_future =
            rt.spawn(ReceivedMessagesBuffer::run_query_output_controller(
                received_messages_buffer,
                received_messages_buffer_output_rx,
            ));

        let mix_traffic_future = rt.spawn(MixTrafficController::run(mix_rx));
        let loop_cover_traffic_future = rt.spawn(NymClient::start_loop_cover_traffic_stream(
            mix_tx.clone(),
            Destination::new(self.address, Default::default()),
            versioned_healthy_topology.clone(),
        ));

        let out_queue_control_future = rt.spawn(NymClient::control_out_queue(
            mix_tx,
            self.input_rx,
            Destination::new(self.address, Default::default()),
            versioned_healthy_topology.clone(),
        ));

        let provider_polling_future = rt.spawn(NymClient::start_provider_polling(
            provider_client,
            poller_input_tx,
        ));

        match self.socket_type {
            SocketType::WebSocket => {
                rt.spawn(ws::start_websocket(
                    self.socket_listening_address,
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self.address,
                    versioned_healthy_topology,
                ));
            }
            SocketType::TCP => {
                rt.spawn(tcp::start_tcpsocket(
                    self.socket_listening_address,
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self.address,
                    versioned_healthy_topology,
                ));
            }
            SocketType::None => (),
        }

        rt.block_on(async {
            let future_results = join!(
                received_messages_buffer_input_controller_future,
                received_messages_buffer_output_controller_future,
                mix_traffic_future,
                loop_cover_traffic_future,
                out_queue_control_future,
                provider_polling_future,
            );

            assert!(
                future_results.0.is_ok()
                    && future_results.1.is_ok()
                    && future_results.2.is_ok()
                    && future_results.3.is_ok()
                    && future_results.4.is_ok()
                    && future_results.5.is_ok()
            );
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on traffic senders
        eprintln!("The client went kaput...");
        Ok(())
    }
}
