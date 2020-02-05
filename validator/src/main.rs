use crate::validator::Config;
use crate::validator::Validator;
use clap::{App, Arg, ArgMatches, SubCommand};
use log::*;
use toml;

pub mod built_info;
mod commands;
mod config;
mod network;
mod services;
mod validator;

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let arg_matches = App::new("Nym Validator")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of Nym Validator")
        .subcommand(commands::init::command_args())
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

    execute(arg_matches);
}

fn run(matches: &ArgMatches) {
    let config = parse_config(matches);
    trace!("read config: {:?}", config);

    let validator = Validator::new(config);
    validator.start()
}

fn parse_config(matches: &ArgMatches) -> Config {
    let config_file_path = matches.value_of("config").unwrap();
    // since this is happening at the very startup, it's fine to panic if file doesn't exist
    let config_content = std::fs::read_to_string(config_file_path).unwrap();
    toml::from_str(&config_content).unwrap()
}

fn execute(matches: ArgMatches) {
    match matches.subcommand() {
        ("init", Some(m)) => commands::init::execute(m),
        ("run", Some(m)) => run(m),
        _ => println!("{}", usage()),
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
