use crate::banner;
use crate::clients::NymClient;
use crate::identity::mixnet;
use crate::persistence::pathfinder::Pathfinder;
use crate::persistence::pemstore::PemStore;
use clap::ArgMatches;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());

    let is_local = matches.is_present("local");

    let id = matches.value_of("id").unwrap().to_string();
    println!("Starting client, local: {:?}", is_local);

    let keypair = read_keypair_from_disk(id);
    let client = NymClient::new(keypair.public_bytes(), is_local);
    client.start().unwrap();
}

fn read_keypair_from_disk(id: String) -> mixnet::KeyPair {
    let pathfinder = Pathfinder::new(id);
    let pem_store = PemStore::new(pathfinder);
    let keypair = pem_store.read();
    keypair
}
