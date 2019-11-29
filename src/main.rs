use tokio::net::TcpListener;
use tokio::prelude::*;
use sphinx::SphinxPacket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let my_address = "127.0.0.1:8081";
    let mut listener = TcpListener::bind(my_address).await?;

    println!("Starting Nym store-and-forward Provider on address {:?}", my_address);
    println!("Waiting for input...");

    loop {
        let (mut inbound, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let _ = match inbound.read(&mut buf).await {
                    Ok(length) if length == 0 =>
                        {
                            println!("Remote connection closed.");
                            return
                        }
                    Ok(_) => {
                        println!("Received a packet!");
                        let packet = SphinxPacket::from_bytes(buf.to_vec()).unwrap();
                        let (unwrapped_packet, _) = packet.process(Default::default());
                        let message = unwrapped_packet.payload.content;
                        println!("Got message: {:?}", String::from_utf8(message).unwrap());
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
