use crate::client::NymClient;
use crate::commands::override_config;
use crate::config::persistance::pathfinder::ClientPathfinder;
use crate::config::Config;
use clap::ArgMatches;
use config::NymConfig;
use pemstore::pemstore::PemStore;

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    let identity_keypair = PemStore::new(ClientPathfinder::new_from_config(&config))
        .read_identity()
        .expect("Failed to read stored identity key files");

    println!(
        "Public identity key: {}\nFor time being, it is identical to address",
        identity_keypair.public_key.to_base58_string()
    );

    let client = NymClient::new(config);
    client.start().unwrap();
}
