use sphinx::{ProcessedPacket, SphinxPacket};
use tokio::prelude::*;
use tokio::runtime::Runtime;
use curve25519_dalek::scalar::Scalar;
use std::net::SocketAddr;

pub struct ServiceProvider {
    network_address: SocketAddr,
    secret_key: Scalar,
}

impl ServiceProvider {
    pub fn new(network_address: SocketAddr, secret_key: Scalar) -> Self {
        ServiceProvider {
            network_address,
            secret_key,
        }
    }

    pub fn start_listening(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the runtime, probably later move it to Provider struct itself?
        let mut rt = Runtime::new()?;

        // Spawn the root task
        rt.block_on(async {
            let my_address = "127.0.0.1:8081";
            let mut listener = tokio::net::TcpListener::bind(my_address).await?;

            println!("Starting Nym store-and-forward Provider on address {:?}", my_address);
            println!("Waiting for input...");

            loop {
                let (mut inbound, _) = listener.accept().await?;

                tokio::spawn(async move {
                    let mut buf = [0; sphinx::PACKET_SIZE];

                    loop {
                        match inbound.read(&mut buf).await {
                            Ok(length) if length == 0 =>
                                {
                                    println!("Remote connection closed.");
                                    return;
                                }
                            Ok(_) => {
                                let packet = SphinxPacket::from_bytes(buf.to_vec()).unwrap();
                                let payload = match packet.process(Default::default()) {
                                    ProcessedPacket::ProcessedPacketFinalHop(_, _, payload) => Some(payload),
                                    _ => None,
                                }.unwrap();
                                let message = payload.get_content();
                            }
                            Err(e) => {
                                println!("failed to read from socket; err = {:?}", e);
                                return;
                            }
                        };
                    }
                });
            }
        })
    }

}

