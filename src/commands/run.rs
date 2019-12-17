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
use std::net::SocketAddrV4;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::{interval_at, Instant};

pub fn execute(matches: &ArgMatches) {
    let is_local = matches.is_present("local");
    println!("Starting client, local: {:?}", is_local);

    // todo: to be taken from config or something
    let my_address = [42u8; 32];
    let is_local = true;
    let client = NymClient::new(my_address, is_local);
    client.start().unwrap();
    // Grab the network topology from the remote directory server
}

// TODO: where do we retrieve this guy from?
fn get_destination() -> Destination {
    Destination {
        address: [42u8; 32],
        identifier: [1u8; 16],
    }
}
