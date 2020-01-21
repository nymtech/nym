use crate::built_info;
use crate::client::mix_traffic::{MixMessage, MixTrafficController};
use crate::client::received_buffer::ReceivedMessagesBuffer;
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

mod cover_traffic_stream;
mod mix_traffic;
mod provider_poller;
pub mod received_buffer;

// TODO: all of those constants should probably be moved to config file
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

// TODO: this will be moved into module responsible for refreshing topology
#[derive(Debug)]
enum TopologyError {
    HealthCheckError,
    NoValidPathsError,
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

    // TODO: this will be moved into module responsible for refreshing topology
    async fn get_compatible_topology(&self) -> Result<Topology, TopologyError> {
        let score_threshold = 0.0;
        info!("Trying to obtain valid, healthy, topology");

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
        let healthcheck_result = healthcheck.do_check().await;

        let healthcheck_scores = match healthcheck_result {
            Err(err) => {
                return Err(TopologyError::HealthCheckError);
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
            return Err(TopologyError::NoValidPathsError);
        }

        Ok(versioned_healthy_topology)
    }

    pub fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting nym client");
        let mut rt = Runtime::new()?;

        // channels for intercomponent communication
        let (mix_tx, mix_rx) = mpsc::unbounded();
        let (poller_input_tx, poller_input_rx) = mpsc::unbounded();
        let (received_messages_buffer_output_tx, received_messages_buffer_output_rx) =
            mpsc::unbounded();

        let initial_topology = match rt.block_on(self.get_compatible_topology()) {
            Ok(topology) => topology,
            Err(err) => {
                panic!("Failed to obtain initial network topology: {:?}", err);
            }
        };

        // this is temporary and assumes there exists only a single provider.
        let provider_client_listener_address: SocketAddr = initial_topology
            .get_mix_provider_nodes()
            .first()
            .expect("Could not get a provider from the supplied network topology, are you using the right directory server?")
            .client_listener;

        let mut provider_poller = provider_poller::ProviderPoller::new(
            poller_input_tx,
            provider_client_listener_address,
            self.address,
            self.auth_token,
        );

        // registration
        if let Err(err) = rt.block_on(provider_poller.perform_initial_registration()) {
            panic!("Failed to perform initial registration: {:?}", err);
        };

        let received_messages_buffer_controllers_future = rt.spawn(
            ReceivedMessagesBuffer::new()
                .start_controllers(poller_input_rx, received_messages_buffer_output_rx),
        );

        let mix_traffic_future = rt.spawn(MixTrafficController::run(mix_rx));
        let loop_cover_traffic_future =
            rt.spawn(cover_traffic_stream::start_loop_cover_traffic_stream(
                mix_tx.clone(),
                Destination::new(self.address, Default::default()),
                initial_topology.clone(),
            ));

        let out_queue_control_future = rt.spawn(NymClient::control_out_queue(
            mix_tx,
            self.input_rx,
            Destination::new(self.address, Default::default()),
            initial_topology.clone(),
        ));

        let provider_polling_future = rt.spawn(provider_poller.start_provider_polling());

        match self.socket_type {
            SocketType::WebSocket => {
                rt.spawn(ws::start_websocket(
                    self.socket_listening_address,
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self.address,
                    initial_topology,
                ));
            }
            SocketType::TCP => {
                rt.spawn(tcp::start_tcpsocket(
                    self.socket_listening_address,
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self.address,
                    initial_topology,
                ));
            }
            SocketType::None => (),
        }

        rt.block_on(async {
            let future_results = join!(
                received_messages_buffer_controllers_future,
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
            );
        });

        // this line in theory should never be reached as the runtime should be permanently blocked on traffic senders
        eprintln!("The client went kaput...");
        Ok(())
    }
}
