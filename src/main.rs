use sphinx::{ProcessedPacket, SphinxPacket};
use tokio::prelude::*;
use tokio::runtime::Runtime;
use curve25519_dalek::scalar::Scalar;
use std::time::{Duration};
use crate::mix_peer::MixPeer;
use crate::MixProcessingError::SphinxRecoveryError;
use sphinx::header::delays::{Delay as SphinxDelay};

mod mix_peer;

// TODO: this will probably need to be moved elsewhere I imagine
#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedFinalHopError,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        SphinxRecoveryError
    }
}

struct ForwardingData<'a> {
    packet: SphinxPacket,
    delay: SphinxDelay,
    recipient: MixPeer<'a>
}

// TODO: this will need to be changed if MixPeer will live longer than our Forwarding Data
impl<'a> ForwardingData<'a> {
    fn new(packet: SphinxPacket, delay: SphinxDelay, recipient: MixPeer<'a>) -> Self {
        ForwardingData {
            packet,
            delay,
            recipient
        }
    }
}

struct PacketProcessor {
}

impl PacketProcessor {
    pub fn process_sphinx_data_packet<'a>(packet_data: &[u8], secret_key: &Scalar) -> Result<ForwardingData<'a>, MixProcessingError> {
        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let (next_packet, next_hop_address, delay) = match packet.process(*secret_key) {
            ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay) => (packet, address, delay),
            _ => return Err(MixProcessingError::ReceivedFinalHopError),
        };

        let next_mix = MixPeer::new(next_hop_address);

        let fwd_data = ForwardingData::new(next_packet, delay, next_mix);
        Ok(fwd_data)
    }

    async fn wait_and_forward(forwarding_data: ForwardingData<'_>) {
        let delay_duration = Duration::from_nanos(forwarding_data.delay.get_value());
        println!("client says to wait for {:?}", delay_duration);
        tokio::time::delay_for(delay_duration).await;
        println!("waited {:?} - time to forward the packet!", delay_duration);

        match forwarding_data.recipient.send(forwarding_data.packet.to_bytes()).await {
            Ok(()) => (),
            Err(e) => {
                println!("failed to write bytes to next mix peer. err = {:?}", e.to_string());
            }
        }
    }
}


// the MixNode will live for whole duration of this program
struct MixNode {
    network_address: &'static str,
    secret_key: Scalar
}

impl MixNode{
    pub fn new(network_address: &'static str, secret_key: Scalar) -> Self {
        MixNode {
            network_address,
            secret_key
        }
    }



    pub fn start_listening(network_address: &str, secret_key: Scalar) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to MixNode itself?
        let mut rt = Runtime::new()?;

        // Spawn the root task
        rt.block_on(async {
            let mut listener = tokio::net::TcpListener::bind(network_address).await?;

            loop {
                let (mut socket, _) = listener.accept().await?;

                tokio::spawn(async move {
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
                                let fwd_data = PacketProcessor::process_sphinx_data_packet(buf.as_ref(), &secret_key).unwrap();
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
                });
            }
        })
    }
}

fn main() {
    let mix = MixNode::new("127.0.0.1:8080", Default::default());
    MixNode::start_listening(mix.network_address, mix.secret_key).unwrap();
}


//
//#[tokio::main]
//async fn main() -> Result<(), Box<dyn std::error::Error>> {
//    let my_address = "127.0.0.1:8080";
//    let mut listener = TcpListener::bind(my_address).await?;
//
//    println!("Starting Nym mixnode on address {:?}", my_address);
//    println!("Waiting for input...");
//
//    loop {
//        let (mut inbound, _) = listener.accept().await?;
//
//        tokio::spawn(async move {
//            let mut buf = [0; 1024 + 333];
//
//            loop {
//                let _ = match inbound.read(&mut buf).await {
//                    Ok(length) if length == 0 =>
//                        {
//                            println!("Remote connection closed.");
//                            return;
//                        }
//                    Ok(length) => {
//                        let packet = SphinxPacket::from_bytes(buf.to_vec()).unwrap();
//                        let next_packet = match packet.process(Default::default()) {
//                            ProcessedPacket::ProcessedPacketForwardHop(packet, _, _) => Some(packet),
//                            _ => None,
//                        }.unwrap();
//
//                        let next_mix = MixPeer::new();
//
//                        match next_mix.send(next_packet.to_bytes()).await {
//                            Ok(()) => length,
//                            Err(e) => {
//                                println!("failed to write bytes to next mix peer. err = {:?}", e.to_string());
//                                return;
//                            }
//                        }
//                    }
//                    Err(e) => {
//                        println!("failed to read from socket; err = {:?}", e);
//                        return;
//                    }
//                };
//            }
//        });
//    }
//}
