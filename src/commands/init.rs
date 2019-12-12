use crate::banner;
use crate::identity::mixnet;
use crate::persistence::pathfinder::Pathfinder;
use crate::persistence::pemstore::PemStore;
use clap::ArgMatches;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap().to_string(); // required for now
    let pathfinder = Pathfinder::new(id);

    println!("Writing keypairs to {:?}...", pathfinder.config_dir);
    let mix_keys = mixnet::KeyPair::new();
    let pem_store = PemStore::new(pathfinder);
    pem_store.write(mix_keys);

    println!("Client configuration completed.\n\n\n")
}
