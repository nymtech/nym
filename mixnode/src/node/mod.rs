use crate::config::persistance::pathfinder::MixNodePathfinder;
use crate::config::Config;
use crate::mix_peer::MixPeer;
use crate::node::metrics::MetricsReporter;
use crypto::encryption;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::SinkExt;
use log::*;
use pemstore::pemstore::PemStore;
use sphinx::header::delays::Delay as SphinxDelay;
use sphinx::{ProcessedPacket, SphinxPacket};
use std::sync::Arc;
use std::time::Duration;
use tokio::prelude::*;
use tokio::runtime::Runtime;

mod metrics;
mod presence;

#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedFinalHopError,
    SphinxProcessingError,
    InvalidHopAddress,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

struct ForwardingData {
    packet: SphinxPacket,
    delay: SphinxDelay,
    recipient: MixPeer,
    sent_metrics_tx: mpsc::Sender<String>,
}

// TODO: this will need to be changed if MixPeer will live longer than our Forwarding Data
impl ForwardingData {
    fn new(
        packet: SphinxPacket,
        delay: SphinxDelay,
        recipient: MixPeer,
        sent_metrics_tx: mpsc::Sender<String>,
    ) -> Self {
        ForwardingData {
            packet,
            delay,
            recipient,
            sent_metrics_tx,
        }
    }
}

// ProcessingData defines all data required to correctly unwrap sphinx packets
struct ProcessingData {
    secret_key: encryption::PrivateKey,
    received_metrics_tx: mpsc::Sender<()>,
    sent_metrics_tx: mpsc::Sender<String>,
}

impl ProcessingData {
    fn new(
        secret_key: encryption::PrivateKey,
        received_metrics_tx: mpsc::Sender<()>,
        sent_metrics_tx: mpsc::Sender<String>,
    ) -> Self {
        ProcessingData {
            secret_key,
            received_metrics_tx,
            sent_metrics_tx,
        }
    }

    fn add_arc_mutex(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }
}

struct PacketProcessor;

impl PacketProcessor {
    pub async fn process_sphinx_data_packet(
        packet_data: &[u8],
        processing_data: Arc<Mutex<ProcessingData>>,
    ) -> Result<ForwardingData, MixProcessingError> {
        // we received something resembling a sphinx packet, report it!
        let processing_data = processing_data.lock().await;
        let mut received_metrics_tx = processing_data.received_metrics_tx.clone();

        // if unwrap failed it means our metrics reporter died, so we should exit application and
        // force restart
        if received_metrics_tx.send(()).await.is_err() {
            error!("failed to send metrics data to the controller - the underlying thread probably died!");
            std::process::exit(1);
        }

        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let (next_packet, next_hop_address, delay) =
            match packet.process(processing_data.secret_key.inner()) {
                Ok(ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay)) => {
                    (packet, address, delay)
                }
                Ok(_) => return Err(MixProcessingError::ReceivedFinalHopError),
                Err(e) => {
                    warn!("Failed to unwrap Sphinx packet: {:?}", e);
                    return Err(MixProcessingError::SphinxProcessingError);
                }
            };

        let next_mix = match MixPeer::new(next_hop_address) {
            Ok(next_mix) => next_mix,
            Err(_) => return Err(MixProcessingError::InvalidHopAddress),
        };

        let fwd_data = ForwardingData::new(
            next_packet,
            delay,
            next_mix,
            processing_data.sent_metrics_tx.clone(),
        );
        Ok(fwd_data)
    }

    async fn wait_and_forward(mut forwarding_data: ForwardingData) {
        let delay_duration = Duration::from_nanos(forwarding_data.delay.get_value());
        tokio::time::delay_for(delay_duration).await;

        if forwarding_data
            .sent_metrics_tx
            .send(forwarding_data.recipient.stringify())
            .await
            .is_err()
        {
            error!("failed to send metrics data to the controller - the underlying thread probably died!");
            std::process::exit(1);
        }

        trace!("RECIPIENT: {:?}", forwarding_data.recipient);
        match forwarding_data
            .recipient
            .send(forwarding_data.packet.to_bytes())
            .await
        {
            Ok(()) => (),
            Err(e) => {
                warn!(
                    "failed to write bytes to next mix peer. err = {:?}",
                    e.to_string()
                );
            }
        }
    }
}

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    sphinx_keypair: encryption::KeyPair,
}

impl MixNode {
    fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
        let sphinx_keypair = PemStore::new(MixNodePathfinder::new_from_config(&config_file))
            .read_encryption()
            .expect("Failed to read stored sphinx key files");
        println!(
            "Public encryption key: {}\nFor time being, it is identical to identity keys",
            sphinx_keypair.public_key().to_base58_string()
        );
        sphinx_keypair
    }

    pub fn new(config: Config) -> Self {
        let sphinx_keypair = Self::load_sphinx_keys(&config);

        MixNode {
            config,
            sphinx_keypair,
        }
    }

    async fn process_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<Mutex<ProcessingData>>,
    ) {
        let mut buf = [0u8; sphinx::PACKET_SIZE];

        // In a loop, read data from the socket and write the data back.
        loop {
            match socket.read(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => {
                    trace!("Remote connection closed.");
                    return;
                }
                Ok(_) => {
                    let fwd_data = match PacketProcessor::process_sphinx_data_packet(
                        buf.as_ref(),
                        processing_data.clone(),
                    )
                    .await
                    {
                        Ok(fwd_data) => fwd_data,
                        Err(e) => {
                            warn!("failed to process sphinx packet: {:?}", e);
                            return;
                        }
                    };
                    PacketProcessor::wait_and_forward(fwd_data).await;
                }
                Err(e) => {
                    warn!("failed to read from socket; err = {:?}", e);
                    return;
                }
            };

            // Write the some data back
            if let Err(e) = socket.write_all(b"foomp").await {
                warn!("failed to write reply to socket; err = {:?}", e);
                return;
            }
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Set up channels
        let (received_tx, received_rx) = mpsc::channel(1024);
        let (sent_tx, sent_rx) = mpsc::channel(1024);

        // Create the runtime, probably later move it to MixNode itself?
        let mut rt = Runtime::new()?;

        // Spawn Tokio tasks as necessary for node functionality
    pub fn start_presence_notifier(&self) {
        let notifier_config = presence::NotifierConfig::new(
            self.config.get_presence_directory_server(),
            self.config.get_announce_address(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_layer(),
            self.config.get_presence_sending_delay(),
        );
        presence::Notifier::new(notifier_config).start(self.runtime.handle())
    }

        let metrics = MetricsReporter::new().add_arc_mutex();
        rt.spawn(MetricsReporter::run_received_metrics_control(
            metrics.clone(),
            received_rx,
        ));
        rt.spawn(MetricsReporter::run_sent_metrics_control(
            metrics.clone(),
            sent_rx,
        ));

        let directory_cfg = directory_client::Config {
            base_url: self.config.get_metrics_directory_server(),
        };

        rt.spawn(MetricsReporter::run_metrics_sender(
            metrics,
            directory_cfg,
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_metrics_sending_delay(),
        ));

        // Spawn the root task
        rt.block_on(async {
            let mut listener =
                tokio::net::TcpListener::bind(self.config.get_listening_address()).await?;
            let processing_data =
                ProcessingData::new(*self.sphinx_keypair.private_key(), received_tx, sent_tx)
                    .add_arc_mutex();

            loop {
                let (socket, _) = listener.accept().await?;

                let thread_processing_data = processing_data.clone();
                tokio::spawn(async move {
                    MixNode::process_socket_connection(socket, thread_processing_data).await;
                });
            }
        })
    }
}
