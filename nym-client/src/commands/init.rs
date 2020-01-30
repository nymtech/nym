use crate::config::persistance::pathfinder::ClientPathfinder;
use clap::ArgMatches;
use crypto::identity::MixIdentityKeyPair;
use pemstore::pemstore::PemStore;

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap().to_string(); // required for now
    let pathfinder = ClientPathfinder::new(id);

    println!("Writing keypairs to {:?}...", pathfinder.config_dir);
    let mix_keys = MixIdentityKeyPair::new();
    let pem_store = PemStore::new(pathfinder);
    pem_store.write_identity(mix_keys).unwrap();

    println!("Client configuration completed.\n\n\n")
}
