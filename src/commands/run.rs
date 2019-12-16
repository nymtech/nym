use crate::clients::directory;
use crate::clients::directory::presence::Topology;
use crate::clients::directory::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::clients::directory::DirectoryClient;
use crate::clients::mix::MixClient;
use crate::clients::provider::ProviderClient;
use crate::clients::NymClient;
use crate::utils::bytes;
use base64;
use clap::ArgMatches;
use curve25519_dalek::montgomery::MontgomeryPoint;
use sphinx::route::Destination;
use sphinx::route::Node as SphinxNode;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::{interval_at, Instant};

pub fn execute(matches: &ArgMatches) {
    let is_local = matches.is_present("local");
    println!("Starting client, local: {:?}", is_local);

    // todo: to be taken from config or something
    let my_address = [42u8; 32];
    let client = NymClient::new(my_address);
    client.start().unwrap();
    // Grab the network topology from the remote directory server
    //    let topology = get_topology(is_local);

    //    // Grab the network topology from the remote directory server
    //    let topology = get_topology();
    //
    //    // Create the runtime, probably later move it to Client struct itself?
    //    let mut rt = Runtime::new().unwrap();
    //
    //    // Spawn the root task
    //    rt.block_on(async {
    //        let start = Instant::now() + Duration::from_nanos(1000);
    //        let mut interval = interval_at(start, Duration::from_millis(1000));
    //        let mut i: usize = 0;
    //        loop {
    //            interval.tick().await;
    ////            let message = format!("Hello, Sphinx {}", i).as_bytes().to_vec();
    ////
    ////            let route_len = 2;
    ////
    ////            // data needed to generate a new Sphinx packet
    ////            let route = route_from(&topology, route_len);
    ////            let destination = get_destination();
    ////            let delays = sphinx::header::delays::generate(route_len);
    ////
    ////            // build the packet
    ////            let packet =
    ////                sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();
    ////
    ////            // send to mixnet
    ////            let mix_client = MixClient::new();
    ////            let result = mix_client.send(packet, route.first().unwrap()).await;
    ////            println!("packet sent:  {:?}", i);
    //            i += 1;
    //
    //            // retrieve messages every now and then
    //            if i % 3 == 0 {
    //                interval.tick().await;
    //                println!("going to retrieve messages!");
    //                let provider_client = ProviderClient::new();
    //                provider_client.retrieve_messages().await.unwrap();
    //            }
    //        }
    //    })
    //    // Create the runtime, probably later move it to Client struct itself?
    //    let mut rt = Runtime::new().unwrap();
    //
    //    // Spawn the root task
    //    rt.block_on(async {
    //        let start = Instant::now() + Duration::from_nanos(1000);
    //        let mut interval = interval_at(start, Duration::from_millis(1000));
    //        let mut i: usize = 0;
    //        loop {
    //            interval.tick().await;
    //            let message = format!("Hello, Sphinx {}", i).as_bytes().to_vec();
    //
    //            let route_len = 2;
    //
    //            // data needed to generate a new Sphinx packet
    //            let route = route_from(&topology, route_len);
    //            let destination = get_destination();
    //            let delays = sphinx::header::delays::generate(route_len);
    //
    //            // build the packet
    //            let packet =
    //                sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();
    //
    //            // send to mixnet
    //            let mix_client = MixClient::new();
    //            mix_client
    //                .send(packet, route.first().unwrap())
    //                .await
    //                .unwrap();
    //            println!("packet sent:  {:?}", i);
    //            i += 1;
    //
    //            // retrieve messages every now and then
    //            if i % 3 == 0 {
    //                interval.tick().await;
    //                println!("going to retrieve messages!");
    //                let provider_client = ProviderClient::new();
    //                provider_client.retrieve_messages().await.unwrap();
    //            }
    //        }
    //    })
}

// TODO: where do we retrieve this guy from?
fn get_destination() -> Destination {
    Destination {
        address: [42u8; 32],
        identifier: [1u8; 16],
    }
}
