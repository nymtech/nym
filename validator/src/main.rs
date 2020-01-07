use crate::validator::config::Config;
use crate::validator::Validator;
use clap::{App, Arg, ArgMatches, SubCommand};
use dotenv;
use log::{error, trace};
use std::process;
use toml;

mod validator;

fn main() {
    // load environment variables from .env file
    // DO NOT USE IN PRODUCTION - REPLACE WITH PROPERLY SET VARIABLES
    if dotenv::dotenv().is_err() {
        eprint!("failed to read .env file - the logging is unlikely to work correctly")
    }

    // if we want to log to file or use different logger, we'd need to replace it here.
    // a better alternative, but way more complex would be `slog` crate - we should
    // perhaps research it at some point.
    pretty_env_logger::init();

    let arg_matches = App::new("Nym Validator")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of Nym Validator")
        .subcommand(
            SubCommand::with_name("run")
                .about("Starts the validator")
                .arg(
                    Arg::with_name("config")
                        .long("config")
                        .help("Location of the validator configuration file")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        error!("{:?}", e);
        process::exit(1);
    }
}

fn run(matches: &ArgMatches) {
    let config = parse_config(matches);
    trace!("read config: {:?}", config);

    let validator = Validator::new(&config);
    validator.start()
}

fn parse_config(matches: &ArgMatches) -> Config {
    let config_file_path = matches.value_of("config").unwrap();
    // since this is happening at the very startup, it's fine to panic if file doesn't exist
    let config_content = std::fs::read_to_string(config_file_path).unwrap();
    toml::from_str(&config_content).unwrap()
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("run", Some(m)) => Ok(run(m)),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    banner() + "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (validator - version {:})

    "#,
        built_info::PKG_VERSION
    )
}
