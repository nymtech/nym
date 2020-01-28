use crate::config::persistance::pathfinder::ClientPathfinder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod persistance;
mod template;

#[derive(Serialize, Deserialize)]
pub struct Config {
    client: Client,
}

impl Default for Config {
    fn default() -> Self {
        unimplemented!()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Client {
    // nym_home_directory specifies absolute path to the home nym Clients directory.
    // It is expected to use default value and hence .toml file should not redefine this field.
    nym_home_directory: String,

    // ID specifies the human readable ID of this particular client.
    // If not provided a pseudorandom id will be generated instead.
    id: String,

    // URL to the directory server.
    directory_server: String,

    // Path to file containing private identity key.
    private_identity_key_file: PathBuf,

    // Path to file containing public identity key.
    public_identity_key_file: PathBuf,

    // mix_apps_directory specifies directory for mixapps, such as a chat client,
    // to store their app-specific data.
    mix_apps_directory: String,

    // provider_id specifies ID of the provider to which the client should send messages.
    // If initially omitted, a random provider will be chosen from the available topology.
    provider_id: String,
}

impl Default for Client {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Client {
    fn new() -> Self {
        Default::default()
    }

    fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    fn with_pathfinder(mut self, pathfinder: ClientPathfinder) -> Self {
        //        pub config_dir: PathBuf,
        //        pub private_mix_key: PathBuf,
        //        pub public_mix_key: PathBuf,
        self
    }
}

pub struct Logging {}

impl Default for Logging {
    fn default() -> Self {
        unimplemented!()
    }
}

#[cfg(test)]
mod client_config {
    use super::*;
}
