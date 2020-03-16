use crate::config::Config;
use clap::ArgMatches;

pub mod init;
pub mod run;

pub(crate) fn override_config(mut config: Config, matches: &ArgMatches) -> Config {
    let mut was_host_overridden = false;
    if let Some(host) = matches.value_of("host") {
        config = config.with_listening_host(host);
        was_host_overridden = true;
    }

    if let Some(port) = matches.value_of("port").map(|port| port.parse::<u16>()) {
        if let Err(err) = port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_listening_port(port.unwrap());
    }

    if let Some(directory) = matches.value_of("directory") {
        config = config.with_custom_directory(directory);
    }

    if let Some(announce_host) = matches.value_of("announce-host") {
        config = config.with_announce_host(announce_host);
    } else if was_host_overridden {
        // make sure our 'announce-host' always defaults to 'host'
        config = config.announce_host_from_listening_host()
    }

    if let Some(announce_port) = matches
        .value_of("announce-port")
        .map(|port| port.parse::<u16>())
    {
        if let Err(err) = announce_port {
            // if port was overridden, it must be parsable
            panic!("Invalid port value provided - {:?}", err);
        }
        config = config.with_announce_port(announce_port.unwrap());
    }

    if let Some(location) = matches.value_of("location") {
        config = config.with_location(location);
    }

    config
}
