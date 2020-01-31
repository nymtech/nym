use crate::config::persistance::pathfinder::ClientPathfinder;
use clap::ArgMatches;
use config::NymConfig;
use crypto::identity::MixIdentityKeyPair;
use pemstore::pemstore::PemStore;

pub fn execute(matches: &ArgMatches) {
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let mut config = crate::config::Config::new(id);

    if let Some(provider_id) = matches.value_of("provider") {
        config = config.with_provider_id(provider_id);
    }

    let mix_identity_keys = MixIdentityKeyPair::new();
    let pathfinder = ClientPathfinder::new_from_config(&config);

    let pem_store = PemStore::new(pathfinder);
    pem_store
        .write_identity(mix_identity_keys)
        .expect("Failed to save identity keys");
    println!("Saved mixnet identity keypair");

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);
    println!("Client configuration completed.\n\n\n")
}
