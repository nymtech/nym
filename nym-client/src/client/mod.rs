use crate::client::mix_traffic::MixTrafficController;
use crate::client::received_buffer::ReceivedMessagesBuffer;
use crate::client::topology_control::TopologyInnerRef;
use crate::config::persistance::pathfinder::ClientPathfinder;
use crate::config::{Config, SocketType};
use crate::sockets::tcp;
use crate::sockets::ws;
use crypto::identity::MixIdentityKeyPair;
use directory_client::presence::Topology;
use futures::channel::mpsc;
use futures::join;
use log::*;
use pemstore::pemstore::PemStore;
use sfw_provider_requests::AuthToken;
use sphinx::route::Destination;
use std::net::SocketAddr;
use tokio::runtime::Runtime;
use topology::NymTopology;

mod cover_traffic_stream;
mod mix_traffic;
mod provider_poller;
mod real_traffic_stream;
pub mod received_buffer;
pub mod topology_control;

pub struct NymClient {
    config: Config,
    identity_keypair: MixIdentityKeyPair,

    // to be used by "send" function or socket, etc
    pub input_tx: mpsc::UnboundedSender<InputMessage>,
    // the other end of the above channel
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
}

#[derive(Debug)]
pub struct InputMessage(pub Destination, pub Vec<u8>);

impl NymClient {
    fn load_identity_keys(config_file: &Config) -> MixIdentityKeyPair {
        let identity_keypair = PemStore::new(ClientPathfinder::new_from_config(&config_file))
            .read_identity()
            .expect("Failed to read stored identity key files");
        println!(
            "Public identity key: {}\nFor time being, it is identical to address",
            identity_keypair.public_key.to_base58_string()
        );
        identity_keypair
    }

    pub fn new(config: Config) -> Self {
        let identity_keypair = Self::load_identity_keys(&config);
        let (input_tx, input_rx) = mpsc::unbounded::<InputMessage>();

        NymClient {
            config,
            identity_keypair,
            input_tx,
            input_rx,
        }
    }

    async fn get_provider_socket_address<T: NymTopology>(
        &self,
        topology_ctrl_ref: TopologyInnerRef<T>,
    ) -> SocketAddr {
        // this is temporary and assumes there exists only a single provider.
        topology_ctrl_ref.read().await.topology.as_ref().unwrap()
            .providers()
            .first()
            .expect("Could not get a compatible provider from the initial network topology, are you using the right directory server?")
            .client_listener
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

        let self_address = self.identity_keypair.public_key().derive_address();

        // generate same type of keys we have as our identity
        let healthcheck_keys = MixIdentityKeyPair::new();

        info!("Trying to obtain initial compatible network topology before starting up rest of modules");
        // TODO: when we switch to our graph topology, we need to remember to change 'Topology' type
        let topology_controller_config = topology_control::TopologyControlConfig::<Topology>::new(
            self.config.get_directory_server(),
            self.config.get_topology_refresh_rate(),
            healthcheck_keys,
            self.config.get_topology_resolution_timeout(),
            self.config.get_number_of_healthcheck_test_packets() as usize,
        );

        let topology_controller = rt.block_on(topology_control::TopologyControl::<Topology>::new(
            topology_controller_config,
        ));

        let provider_client_listener_address =
            rt.block_on(self.get_provider_socket_address(topology_controller.get_inner_ref()));

        let mut provider_poller = provider_poller::ProviderPoller::new(
            poller_input_tx,
            provider_client_listener_address,
            self_address.clone(),
            self.config
                .get_provider_auth_token()
                .map(|str_token| AuthToken::try_from_base58_string(str_token).ok())
                .unwrap_or(None),
            self.config.get_fetch_message_delay(),
        );

        // registration
        if let Err(err) = rt.block_on(provider_poller.perform_initial_registration()) {
            panic!("Failed to perform initial provider registration: {:?}", err);
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
                Destination::new(self_address.clone(), Default::default()),
                topology_controller.get_inner_ref(),
                self.config.get_loop_cover_traffic_average_delay(),
                self.config.get_average_packet_delay(),
            ));

        // cloning arguments required by OutQueueControl; required due to move
        let input_rx = self.input_rx;
        let topology_ref = topology_controller.get_inner_ref();
        let average_packet_delay = self.config.get_average_packet_delay();
        let message_sending_average_delay = self.config.get_message_sending_average_delay();
        let self_address_clone = self_address.clone();
        // future constantly pumping traffic at some specified average rate
        // if a real message is available on 'input_rx' that might have been received from say
        // the websocket, the real message is used, otherwise a loop cover message is generated
        // the pumped traffic goes to the MixTrafficController
        let out_queue_control_future = rt.spawn(async move {
            real_traffic_stream::OutQueueControl::new(
                mix_tx,
                input_rx,
                Destination::new(self_address_clone, Default::default()),
                topology_ref,
                average_packet_delay,
                message_sending_average_delay,
            )
            .run_out_queue_control()
            .await
        });

        // future constantly trying to fetch any received messages from the provider
        // the received messages are sent to ReceivedMessagesBuffer to be available to rest of the system
        let provider_polling_future = rt.spawn(provider_poller.start_provider_polling());

        match self.config.get_socket_type() {
            SocketType::WebSocket => {
                rt.spawn(ws::start_websocket(
                    self.config.get_listening_port(),
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self_address,
                    topology_controller.get_inner_ref(),
                ));
            }
            SocketType::TCP => {
                rt.spawn(tcp::start_tcpsocket(
                    self.config.get_listening_port(),
                    self.input_tx,
                    received_messages_buffer_output_tx,
                    self_address,
                    topology_controller.get_inner_ref(),
                ));
            }
            SocketType::None => (),
        }

        // future responsible for periodically polling directory server and updating
        // the current global view of topology
        let topology_refresher_future = rt.spawn(topology_controller.run_refresher());

        rt.block_on(async {
            let future_results = join!(
                received_messages_buffer_controllers_future,
                mix_traffic_future,
                loop_cover_traffic_future,
                out_queue_control_future,
                provider_polling_future,
                topology_refresher_future,
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
        error!("The client went kaput...");
        Ok(())
    }
}
