use crate::commands::override_config;
use crate::config::Config;
use crate::validator;
use crate::validator::Validator;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the validator")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-validator we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym-validator configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the validator is sending presence to")
                .takes_value(true),
        )
}

fn parse_old_config(matches: &ArgMatches) -> validator::Config {
    let config_file_path = matches.value_of("config").unwrap();
    // since this is happening at the very startup, it's fine to panic if file doesn't exist
    let config_content = std::fs::read_to_string(config_file_path).unwrap();
    toml::from_str(&config_content).unwrap()
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting sfw-provider {}...", id);

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    let old_config = parse_old_config(matches);

    let validator = Validator::new(old_config);
    validator.start()
}
