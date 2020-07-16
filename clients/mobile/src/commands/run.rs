// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::commands::override_config;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};

use crate::node::MixNode;
use config::NymConfig;
use crypto::encryption;
use pemstore::pemstore::PemStore;

fn show_binding_warning(address: String) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this note if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
    let sphinx_keypair = PemStore::new(MixNodePathfinder::new_from_config(&config_file))
        .read_encryption()
        .expect("Failed to read stored sphinx key files");
    println!(
        "Public key: {}\n",
        sphinx_keypair.public_key().to_base58_string()
    );
    sphinx_keypair
}

pub fn execute(id: String, host: String) {
    println!("Starting mixnode {}", id);
    
    let mut config_origin = crate::config::Config::new(&id, 3);
    let config_file = config_origin.get_config_file_save_location();
    let mut config =
        Config::load_from_file(Some(config_file), Some(&id))
            .expect("Failed to load config file");

    config = override_config(config, host);

    let sphinx_keypair = load_sphinx_keys(&config);

    let listening_ip_string = config.get_listening_address().ip().to_string();
    if special_addresses().contains(&listening_ip_string.as_ref()) {
        show_binding_warning(listening_ip_string);
    }

    println!(
        "Directory server [presence]: {}",
        config.get_presence_directory_server()
    );
    println!(
        "Directory server [metrics]: {}",
        config.get_metrics_directory_server()
    );

    println!(
        "Listening for incoming packets on {}",
        config.get_listening_address()
    );
    println!(
        "Announcing the following socket address: {}",
        config.get_announce_address()
    );

    MixNode::new(config, sphinx_keypair).run();
}
