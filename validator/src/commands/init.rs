use crate::commands::override_config;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("init")
        .about("Initialise the validator")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-validator we want to create config for.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the validator is sending presence to and uses for mix mining")
                .takes_value(true),
        )
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();
    println!("Initialising validator {}...", id);

    let mut config = crate::config::Config::new(id);

    config = override_config(config, matches);

    let config_save_location = config.get_config_file_save_location();
    config
        .save_to_file(None)
        .expect("Failed to save the config file");
    println!("Saved configuration file to {:?}", config_save_location);

    println!("Validator configuration completed.\n\n\n")
}
