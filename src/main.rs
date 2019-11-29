mod clients;

use tokio::prelude::*;

use crate::clients::mix::MixClient;
use crate::clients::directory::DirectoryClient;

#[tokio::main]
async fn main() {
    let message = "Hello, Sphinx!".as_bytes().to_vec();

    // set up the route
    let directory = DirectoryClient::new();
    let route = directory.get_mixes();
    let destination = directory.get_destination();
    let delays = sphinx::header::delays::generate(2);

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();

    // send to mixnet
    let mix_client = MixClient::new();
    let result = mix_client.send(packet, route.first().unwrap()).await;
    println!("packet sent");
}


