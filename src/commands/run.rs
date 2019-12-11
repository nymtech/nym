use crate::clients::directory;
use crate::clients::directory::requests::health_check_get::HealthCheckRequester;
use crate::clients::directory::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::clients::directory::DirectoryClient;
use crate::clients::mix::MixClient;
use base64;
use clap::ArgMatches;
use curve25519_dalek::montgomery::MontgomeryPoint;
use sphinx::route::Destination;
use sphinx::route::Node as SphinxNode;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::{interval_at, Instant};

pub fn execute(matches: &ArgMatches) {
    let custom_cfg = matches.value_of("customCfg");
    println!(
        "Going to start client with custom config of: {:?}",
        custom_cfg
    );

    // Create the runtime, probably later move it to Client struct itself?
    let mut rt = Runtime::new().unwrap();

    // Spawn the root task
    rt.block_on(async {
        let start = Instant::now() + Duration::from_nanos(1000);
        let mut interval = interval_at(start, Duration::from_millis(5000));
        let mut i: usize = 0;
        loop {
            interval.tick().await;
            let message = format!("Hello, Sphinx {}", i).as_bytes().to_vec();

            let directory_config = directory::Config {
                base_url: "https://directory.nymtech.net".to_string(),
            };
            let directory = directory::Client::new(directory_config);

            let topology = directory
                .presence_topology
                .get()
                .expect("Failed to retrieve network topology.");
            let mut route = vec![];
            let route_len = 3;
            let nodes = topology.mix_nodes.iter();
            for mix in nodes.take(route_len) {
                let mut address_vector = mix.host.as_bytes().to_vec();
                if address_vector.len() != 32 {
                    address_vector.resize(32, 0)
                }
                assert!(address_vector.len() <= 32);
                let mut address_bytes = [0; 32];
                address_bytes.copy_from_slice(&address_vector[..]);

                let key_bytes = base64::decode_config(&mix.pub_key, base64::URL_SAFE).unwrap();
                assert!(key_bytes.len() == 32);
                let mut arr = [0; 32];
                arr.copy_from_slice(&key_bytes);
                let mp = MontgomeryPoint(arr);
                let sphinx_node = SphinxNode {
                    address: address_bytes,
                    pub_key: mp,
                };
                route.push(sphinx_node);
            }
            let destination = get_destination();
            let delays = sphinx::header::delays::generate(route_len);

            // build the packet
            let packet =
                sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();
            //
            // send to mixnet
            let mix_client = MixClient::new();
            let result = mix_client.send(packet, route.first().unwrap()).await;
            println!("packet sent:  {:?}", i);
            i += 1;
        }
    })
}

fn zero_pad_to(bytes: Vec<u8>, length: usize) -> Vec<u8> {
    vec![0u8; length]
}

// TODO: where do we retrieve this guy from?
fn get_destination() -> Destination {
    Destination {
        address: [0u8; 32],
        identifier: [0u8; 16],
    }
}
