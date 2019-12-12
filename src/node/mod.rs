use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use curve25519_dalek::scalar::Scalar;
use sphinx::header::delays::Delay as SphinxDelay;
use sphinx::{ProcessedPacket, SphinxPacket};
use tokio::prelude::*;
use tokio::runtime::Runtime;

use crate::mix_peer::MixPeer;

pub mod runner;

// TODO: this will probably need to be moved elsewhere I imagine
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
}

// TODO: this will need to be changed if MixPeer will live longer than our Forwarding Data
impl ForwardingData {
    fn new(packet: SphinxPacket, delay: SphinxDelay, recipient: MixPeer) -> Self {
        ForwardingData {
            packet,
            delay,
            recipient,
        }
    }
}

// ProcessingData defines all data required to correctly unwrap sphinx packets
struct ProcessingData {
    secret_key: Scalar,
}

impl ProcessingData {
    fn new(secret_key: Scalar) -> Self {
        ProcessingData { secret_key }
    }

    fn add_arc_rwlock(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }
}

struct PacketProcessor {}

impl PacketProcessor {
    pub fn process_sphinx_data_packet(
        packet_data: &[u8],
        processing_data: Arc<RwLock<ProcessingData>>,
    ) -> Result<ForwardingData, MixProcessingError> {
        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let (next_packet, next_hop_address, delay) =
            match packet.process(processing_data.read().unwrap().secret_key) {
                ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay) => {
                    (packet, address, delay)
                }
                _ => return Err(MixProcessingError::ReceivedFinalHopError),
            };

        let next_mix = MixPeer::new(next_hop_address);

        let fwd_data = ForwardingData::new(next_packet, delay, next_mix);
        Ok(fwd_data)
    }

    async fn wait_and_forward(forwarding_data: ForwardingData) {
        let delay_duration = Duration::from_nanos(forwarding_data.delay.get_value());
        println!("client says to wait for {:?}", delay_duration);
        tokio::time::delay_for(delay_duration).await;
        println!("waited {:?} - time to forward the packet!", delay_duration);

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
    network_address: SocketAddr,
    secret_key: Scalar,
    layer: usize,
}

impl MixNode {
    pub fn new(network_address: SocketAddr, secret_key: Scalar, layer: usize) -> Self {
        MixNode {
            network_address,
            secret_key,
            layer,
        }
    }

    async fn process_socket_connection(
        mut socket: tokio::net::TcpStream,
        processing_data: Arc<RwLock<ProcessingData>>,
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

    pub fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to MixNode itself?
        let mut rt = Runtime::new()?;

        // Spawn the root task
        rt.block_on(async {
            let mut listener = tokio::net::TcpListener::bind(self.network_address).await?;
            let processing_data = ProcessingData::new(self.secret_key).add_arc_rwlock();

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
