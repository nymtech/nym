use crate::built_info;
use crate::sockets::tcp;
use crate::sockets::ws;
use crate::utils;
use directory_client::presence::Topology;
use futures::channel::{mpsc, oneshot};
use futures::join;
use futures::lock::Mutex as FMutex;
use futures::select;
use futures::{SinkExt, StreamExt};
use log::*;
use sfw_provider_requests::AuthToken;
use sphinx::route::{Destination, DestinationAddressBytes};
use sphinx::SphinxPacket;
use std::io;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use topology::NymTopology;

const LOOP_COVER_AVERAGE_DELAY: f64 = 0.5;
// seconds
const MESSAGE_SENDING_AVERAGE_DELAY: f64 = 0.5;
//  seconds;
const FETCH_MESSAGES_DELAY: f64 = 1.0; // seconds;

// provider-poller sends polls service provider; receives messages
// provider-poller sends (TX) to ReceivedBufferController (RX)
// ReceivedBufferController sends (TX) to ... ??Client??
// outQueueController sends (TX) to TrafficStreamController (RX)
// TrafficStreamController sends messages to mixnet
// ... ??Client?? sends (TX) to outQueueController (RX)
// Loop cover traffic stream just sends messages to mixnet without any channel communication

struct MixMessage(SocketAddr, SphinxPacket);

struct MixTrafficController;

impl MixTrafficController {
    // this was way more difficult to implement than what this code may suggest...
    async fn run(mut rx: mpsc::UnboundedReceiver<MixMessage>) {
        let mix_client = mix_client::MixClient::new();
        while let Some(mix_message) = rx.next().await {
            info!(
                "[MIX TRAFFIC CONTROL] - got a mix_message for {:?}",
                mix_message.0
            );
            let send_res = mix_client.send(mix_message.1, mix_message.0).await;
            match send_res {
                Ok(_) => {
                    print!(".");
                    io::stdout().flush().ok().expect("Could not flush stdout");
                }
                Err(e) => error!("We failed to send the message :( - {:?}", e),
            };
        }
    }
}

pub type BufferResponse = oneshot::Sender<Vec<Vec<u8>>>;

struct ReceivedMessagesBuffer {
    messages: Vec<Vec<u8>>,
}

impl ReceivedMessagesBuffer {
    fn add_arc_futures_mutex(self) -> Arc<FMutex<Self>> {
        Arc::new(FMutex::new(self))
    }

    fn new() -> Self {
        ReceivedMessagesBuffer {
            messages: Vec::new(),
        }
    }

    async fn add_new_messages(buf: Arc<FMutex<Self>>, msgs: Vec<Vec<u8>>) {
        info!("Adding new messages to the buffer! {:?}", msgs);
        let mut unlocked = buf.lock().await;
        unlocked.messages.extend(msgs);
    }

    async fn run_poller_input_controller(
        buf: Arc<FMutex<Self>>,
        mut poller_rx: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
    ) {
        while let Some(new_messages) = poller_rx.next().await {
            ReceivedMessagesBuffer::add_new_messages(buf.clone(), new_messages).await;
        }
    }

    async fn acquire_and_empty(buf: Arc<FMutex<Self>>) -> Vec<Vec<u8>> {
        let mut unlocked = buf.lock().await;
        std::mem::replace(&mut unlocked.messages, Vec::new())
    }

    async fn run_query_output_controller(
        buf: Arc<FMutex<Self>>,
        mut query_receiver: mpsc::UnboundedReceiver<BufferResponse>,
    ) {
        while let Some(request) = query_receiver.next().await {
            let messages = ReceivedMessagesBuffer::acquire_and_empty(buf.clone()).await;
            // if this fails, the whole application needs to blow
            // because currently only this thread would fail
            request.send(messages).unwrap();
        }
    }
}

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
            tx.send(MixMessage(cover_message.0, cover_message.1))
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
                .send(MixMessage(traffic_message.0, traffic_message.1))
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
