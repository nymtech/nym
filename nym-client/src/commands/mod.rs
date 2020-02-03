use crate::config::{Config, SocketType};
use clap::ArgMatches;

pub mod init;
pub mod run;

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    if let Some(directory) = matches.value_of("directory") {
        config = config.with_custom_directory(directory);
    }

    if let Some(provider_id) = matches.value_of("provider") {
        config = config.with_provider_id(provider_id);
    }

    if let Some(socket_type) = matches.value_of("socket-type") {
        config = config.with_socket(SocketType::from_string(socket_type));
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_port(port.unwrap());
    }

    config
}
