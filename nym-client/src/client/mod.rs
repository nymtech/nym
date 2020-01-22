use crate::built_info;
use crate::client::mix_traffic::MixTrafficController;
use crate::client::received_buffer::ReceivedMessagesBuffer;
use crate::sockets::tcp;
use crate::sockets::ws;
use directory_client::presence::Topology;
use futures::channel::mpsc;
use futures::join;
use log::*;
use sfw_provider_requests::AuthToken;
use sphinx::route::{Destination, DestinationAddressBytes};
use std::net::SocketAddr;
use tokio::runtime::Runtime;
use topology::NymTopology;

mod cover_traffic_stream;
mod mix_traffic;
mod provider_poller;
mod real_traffic_stream;
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

    // to be used by "send" function or socket, etc
    pub input_tx: mpsc::UnboundedSender<InputMessage>,

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
                error!("Error while performing the healtcheck: {:?}", err);
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

        // channels for inter-component communication

        // mix_tx is the transmitter for any component generating sphinx packets that are to be sent to the mixnet
        // they are used by cover traffic stream and real traffic stream
        // mix_rx is the receiver used by MixTrafficController that sends the actual traffic
        let (mix_tx, mix_rx) = mpsc::unbounded();

        // poller_input_tx is the transmitter of messages fetched from the provider - used by ProviderPoller
        // poller_input_rx is the receiver for said messages - used by ReceivedMessagesBuffer
        let (poller_input_tx, poller_input_rx) = mpsc::unbounded();

        // received_messages_buffer_output_tx is the transmitter for *REQUESTS* for messages contained in ReceivedMessagesBuffer - used by sockets
        // the requests contain a oneshot channel to send a reply on
        // received_messages_buffer_output_rx is the received for the said requests - used by ReceivedMessagesBuffer
        let (received_messages_buffer_output_tx, received_messages_buffer_output_rx) =
            mpsc::unbounded();

        // get initial topology; already filtered by health and version
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

        // setup all of futures for the components running on the client

        // buffer controlling all messages fetched from provider
        // required so that other components would be able to use them (say the websocket)
        let received_messages_buffer_controllers_future = rt.spawn(
            ReceivedMessagesBuffer::new()
                .start_controllers(poller_input_rx, received_messages_buffer_output_rx),
        );

        // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
        let mix_traffic_future = rt.spawn(MixTrafficController::run(mix_rx));

        // future constantly pumping loop cover traffic at some specified average rate
        // the pumped traffic goes to the MixTrafficController
        let loop_cover_traffic_future =
            rt.spawn(cover_traffic_stream::start_loop_cover_traffic_stream(
                mix_tx.clone(),
                Destination::new(self.address, Default::default()),
                initial_topology.clone(),
            ));

        // cloning arguments required by OutQueueControl; required due to move
        let topology_clone = initial_topology.clone();
        let self_address = self.address;
        let input_rx = self.input_rx;

        // future constantly pumping traffic at some specified average rate
        // if a real message is available on 'input_rx' that might have been received from say
        // the websocket, the real message is used, otherwise a loop cover message is generated
        // the pumped traffic goes to the MixTrafficController
        let out_queue_control_future = rt.spawn(async move {
            real_traffic_stream::OutQueueControl::new(
                mix_tx,
                input_rx,
                Destination::new(self_address, Default::default()),
                topology_clone,
            )
            .run_out_queue_control()
            .await
        });

        // future constantly trying to fetch any received messages from the provider
        // the received messages are sent to ReceivedMessagesBuffer to be available to rest of the system
        let provider_polling_future = rt.spawn(provider_poller.start_provider_polling());

        // a temporary workaround for starting socket listener of specified type
        // in the future the actual socket handler should start THIS client instead
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
