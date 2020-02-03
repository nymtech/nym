use clap::{App, ArgMatches};
use log::*;
use std::process;

pub mod built_info;
mod commands;
mod config;
mod mix_peer;
mod node;

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    println!("{}", banner());

    let arg_matches = App::new("Nym Mixnode")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of the Loopix-based Mixnode")
        .subcommand(commands::init::command_args())
        .subcommand(commands::run::command_args())
        .get_matches();

    execute(arg_matches);
}

fn execute(matches: ArgMatches) {
    match matches.subcommand() {
        ("init", Some(m)) => commands::init::execute(m),
        ("run", Some(m)) => node::runner::start(m),
        _ => println!("{}", usage()),
    }
}

fn usage() -> &'static str {
    "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (mixnode - version {:})

    "#,
        built_info::PKG_VERSION
    )
}
