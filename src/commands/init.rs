use crate::banner;
use crate::identity::mixnet;
use crate::persistence::pemstore::PemStore;
use clap::ArgMatches;
use dirs;

pub fn execute(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Initialising client...");

    let id = matches.value_of("id").unwrap(); // required for now
    let os_config_dir = dirs::config_dir().unwrap(); // grabs the OS default config dir
    let nym_client_config_dir = os_config_dir.join("nym").join("clients").join(id);

    println!("Writing keypairs to {:?}...", nym_client_config_dir);
    let mix_keys = mixnet::KeyPair::new();
    let pem_store = PemStore::new();
    pem_store.write(mix_keys, nym_client_config_dir);

    println!("Client configuration completed.\n\n\n")
}
