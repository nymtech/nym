use tokio::net::TcpListener;
use tokio::prelude::*;
use crate::mix_peer::MixPeer;
use sphinx::{SphinxPacket, ProcessedPacket};

mod mix_peer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let my_address = "127.0.0.1:8080";
    let mut listener = TcpListener::bind(my_address).await?;

    println!("Starting Nym mixnode on address {:?}", my_address);
    println!("Waiting for input...");

    loop {
        let (mut inbound, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024 + 333];

            loop {
                let _ = match inbound.read(&mut buf).await {
                    Ok(length) if length == 0 =>
                        {
                            println!("Remote connection closed.");
                            return
                        }
                    Ok(length) => {
                        let packet = SphinxPacket::from_bytes(buf.to_vec()).unwrap();
                        let next_packet = match packet.process(Default::default()){
                            ProcessedPacket::ProcessedPacketForwardHop(packet,_,_) => Some(packet) ,
                            _ => None,
                        }.unwrap();

                        let next_mix = MixPeer::new();

                        match next_mix.send(next_packet.to_bytes()).await {
                            Ok(()) => length,
                            Err(e) => {
                                println!("failed to write bytes to next mix peer. err = {:?}", e.to_string());
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        println!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };
            }
        });
    }
}
