// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::client_handling::websocket::listener::Listener;
use log::*;

mod client_handling;
mod mixnet_client;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    let addr = "127.0.0.1:1793".parse().unwrap();
    info!("Listening on: {}", addr);

    let (dummy_clients_handler_tx, _) = futures::channel::mpsc::unbounded();
    Listener::new(addr, dummy_clients_handler_tx).start();

        tokio::spawn(accept_connection(peer, stream, Arc::clone(&client_ref)));
    }
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
