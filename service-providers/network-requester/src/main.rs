// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{App, Arg, ArgMatches};

use network_defaults::DEFAULT_WEBSOCKET_LISTENING_PORT;

mod allowed_hosts;
mod connection;
mod core;
mod statistics;
#[cfg(feature = "stats-service")]
mod storage;
mod websocket;

const OPEN_PROXY_ARG: &str = "open-proxy";
const WS_PORT: &str = "websocket-port";
const DESCRIPTION: &str = "description";

fn parse_args<'a>() -> ArgMatches<'a> {
    App::new("Nym Network Requester")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Nymtech")
        .arg(
            Arg::with_name(OPEN_PROXY_ARG)
                .help("specifies whether this network requester should run in 'open-proxy' mode")
                .long(OPEN_PROXY_ARG)
                .short("o"),
        )
        .arg(
            Arg::with_name(WS_PORT)
                .help("websocket port to bind to")
                .long(WS_PORT)
                .short("p")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(DESCRIPTION)
                .help("service description")
                .long(DESCRIPTION)
                .short("d")
                .takes_value(true)
                .default_value("undefined"),
        )
        .get_matches()
}

#[tokio::main]
async fn main() {
    setup_logging();
    let matches = parse_args();

    let open_proxy = matches.is_present(OPEN_PROXY_ARG);
    if open_proxy {
        println!("\n\nYOU HAVE STARTED IN 'OPEN PROXY' MODE. ANYONE WITH YOUR CLIENT ADDRESS CAN MAKE REQUESTS FROM YOUR MACHINE. PLEASE QUIT IF YOU DON'T UNDERSTAND WHAT YOU'RE DOING.\n\n");
    }

    let uri = format!(
        "ws://localhost:{}",
        matches
            .value_of(WS_PORT)
            .unwrap_or(&DEFAULT_WEBSOCKET_LISTENING_PORT.to_string())
    );

    let description = matches.value_of(DESCRIPTION).unwrap().to_string();
    println!("Starting socks5 service provider:");
    let mut server = core::ServiceProvider::new(uri, description, open_proxy);
    server.run().await;
}

fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .init();
}
