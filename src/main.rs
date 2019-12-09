mod clients;

use tokio::prelude::*;

use crate::clients::mix::MixClient;
use crate::clients::directory;
use std::time::{ Duration};
use tokio::time::{interval_at, Instant};

#[tokio::main]
async fn main() {
    let start = Instant::now() + Duration::from_nanos(1000);
    let mut interval = interval_at(start, Duration::from_nanos(1000));
    let mut i = 0;
    loop {
        interval.tick().await;
        let message = format!("Hello, Sphinx {}", i).as_bytes().to_vec();

        // set up the route
        let directory = directory::Client::new();
        let route = directory.get_mixes();
        let destination = directory.get_destination();
        let delays = sphinx::header::delays::generate(2);

        // build the packet
        let packet = sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();

        // send to mixnet
        let mix_client = MixClient::new();
        let result = mix_client.send(packet, route.first().unwrap()).await;
        println!("packet sent:  {:?}", i);
        i += 1;
    }
}