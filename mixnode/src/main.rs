use clap::{App, Arg, ArgMatches, SubCommand};
use std::process;

mod mix_peer;
mod node;

fn main() {
    let arg_matches = App::new("Nym Mixnode")
        .version(built_info::PKG_VERSION)
        .author("Nymtech")
        .about("Implementation of the Loopix-based Mixnode")
        .subcommand(
            SubCommand::with_name("run")
                .about("Starts the mixnode")
                .arg(
                    Arg::with_name("host")
                        .long("host")
                        .help("The custom host on which the mixnode will be running")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .help("The port on which the mixnode will be listening")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("layer")
                        .long("layer")
                        .help("The mixnet layer of this particular node")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("directory")
                        .long("directory")
                        .help("Address of the directory server the node is sending presence and metrics to")
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        println!("{}", e);
        process::exit(1);
    }
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("run", Some(m)) => Ok(node::runner::start(m)),
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

             (mixnode - version {:})

    "#,
        built_info::PKG_VERSION
    )
}
