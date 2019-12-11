use crate::clients::directory;
use crate::clients::directory::requests::health_check_get::HealthCheckRequester;
use crate::clients::directory::requests::presence_topology_get::PresenceTopologyGetRequester;
use crate::clients::directory::DirectoryClient;
use clap::ArgMatches;
use sphinx::route::Destination;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::{interval_at, Instant};

pub fn run(matches: &ArgMatches) {
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

            // set up the route
            let directory_config = directory::Config {
                base_url: "https://directory.nymtech.net".to_string(),
            };
            let directory = directory::Client::new(directory_config);

            // make sure the Directory server is in fact running, panic if not
            directory
                .health_check
                .get()
                .expect("Directory health check failed, is the Directory server running?");

            let topology = directory
                .presence_topology
                .get()
                .expect("Failed to retrieve network topology.");
            let route = topology.mix_nodes;
            let destination = get_destination();
            let delays = sphinx::header::delays::generate(2);

            // build the packet
            //            let packet = sphinx::SphinxPacket::new(message, &route[..], &destination, &delays).unwrap();
            //
            //            // send to mixnet
            //            let mix_client = MixClient::new();
            //            let result = mix_client.send(packet, route.first().unwrap()).await;
            //            println!("packet sent:  {:?}", i);
            //            i += 1;
        }
    })
}

// TODO: where do we retrieve this guy from?
fn get_destination() -> Destination {
    Destination {
        address: [0u8; 32],
        identifier: [0u8; 16],
    }
}
