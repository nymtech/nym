use crate::mix_peer::MixPeer;
use crate::node;
use crate::node::metrics::MetricsReporter;
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::SinkExt;
use sphinx::header::delays::Delay as SphinxDelay;
use sphinx::{ProcessedPacket, SphinxPacket};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::prelude::*;
use tokio::runtime::Runtime;

mod metrics;
mod presence;
pub mod runner;

pub struct Config {
    announce_address: String,
    directory_server: String,
    layer: usize,
    public_key: MontgomeryPoint,
    secret_key: Scalar,
    socket_address: SocketAddr,
}

impl Config {
    pub fn public_key_string(&self) -> String {
        let key_bytes = self.public_key.to_bytes().to_vec();
        base64::encode_config(&key_bytes, base64::URL_SAFE)
    }
}

#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedFinalHopError,
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
    secret_key: Scalar,
    received_metrics_tx: mpsc::Sender<()>,
    sent_metrics_tx: mpsc::Sender<String>,
}

impl ProcessingData {
    fn new(
        secret_key: Scalar,
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
        let mut received_sender = processing_data.received_metrics_tx.clone();

        received_sender.send(()).await.unwrap();

        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let (next_packet, next_hop_address, delay) =
            match packet.process(processing_data.secret_key) {
                ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay) => {
                    (packet, address, delay)
                }
                _ => return Err(MixProcessingError::ReceivedFinalHopError),
            };

        let next_mix = MixPeer::new(next_hop_address);

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
        forwarding_data
            .sent_metrics_tx
            .send(forwarding_data.recipient.to_string())
            .await
            .unwrap();

        println!("RECIPIENT: {:?}", forwarding_data.recipient);
        match forwarding_data
            .recipient
            .send(forwarding_data.packet.to_bytes())
            .await
        {
            Ok(()) => (),
            Err(e) => {
                println!(
                    "failed to write bytes to next mix peer. err = {:?}",
                    e.to_string()
                );
            }
        }
    }
}

// the MixNode will live for whole duration of this program
pub struct MixNode {
    directory_server: String,
    network_address: SocketAddr,
    public_key: MontgomeryPoint,
    secret_key: Scalar,
    // TODO: use it later to enforce forward travel
    //    layer: usize,
}

impl MixNode {
    pub fn new(config: &Config) -> Self {
        MixNode {
            directory_server: config.directory_server.clone(),
            network_address: config.socket_address,
            secret_key: config.secret_key,
            public_key: config.public_key,
            //            layer: config.layer,
        }
    }

    async fn process_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<Mutex<ProcessingData>>,
    ) {
        // NOTE: processing_data is copied here!!
        let mut buf = [0u8; sphinx::PACKET_SIZE];

        // In a loop, read data from the socket and write the data back.
        loop {
            match socket.read(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => {
                    println!("Remote connection closed.");
                    return;
                }
                Ok(_) => {
                    let fwd_data = PacketProcessor::process_sphinx_data_packet(
                        buf.as_ref(),
                        processing_data.clone(),
                    )
                    .await
                    .unwrap();
                    PacketProcessor::wait_and_forward(fwd_data).await;
                }
                Err(e) => {
                    println!("failed to read from socket; err = {:?}", e);
                    return;
                }
            };

            // Write the some data back
            if let Err(e) = socket.write_all(b"foomp").await {
                println!("failed to write reply to socket; err = {:?}", e);
                return;
            }
        }
    }

    pub fn start(&self, config: node::Config) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to MixNode itself?
        let mut rt = Runtime::new()?;

        let (received_tx, received_rx) = mpsc::channel(1024);
        let (sent_tx, sent_rx) = mpsc::channel(1024);

        let directory_cfg = directory_client::Config {
            base_url: self.directory_server.clone(),
        };
        let pub_key_str =
            base64::encode_config(&self.public_key.to_bytes().to_vec(), base64::URL_SAFE);

        rt.spawn({
            let presence_notifier = presence::Notifier::new(&config);
            presence_notifier.run()
        });

        let metrics = MetricsReporter::new().add_arc_mutex();
        rt.spawn(MetricsReporter::run_received_metrics_control(
            metrics.clone(),
            received_rx,
        ));
        rt.spawn(MetricsReporter::run_sent_metrics_control(
            metrics.clone(),
            sent_rx,
        ));
        rt.spawn(MetricsReporter::run_metrics_sender(
            metrics,
            directory_cfg,
            pub_key_str,
        ));

        // Spawn the root task
        rt.block_on(async {
            let mut listener = tokio::net::TcpListener::bind(self.network_address).await?;
            let processing_data =
                ProcessingData::new(self.secret_key, received_tx, sent_tx).add_arc_mutex();

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
