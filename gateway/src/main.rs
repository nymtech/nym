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

use crate::client_handling::clients_handler::ClientsHandler;
use crate::client_handling::websocket;
use crate::mixnet_handling::receiver::packet_processing::PacketProcessor;
use crate::storage::ClientStorage;
use log::*;
use std::sync::Arc;

mod client_handling;
mod mixnet_client;
mod mixnet_handling;
pub(crate) mod storage;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    let addr = "127.0.0.1:1793".parse().unwrap();
    info!("Listening on: {}", addr);

    let (dummy_clients_handler_tx, dummy_clients_handler_rx) = futures::channel::mpsc::unbounded();
    let client_storage = ClientStorage::new(42, 42, "foomp".into());
    let dummy_keypair = crypto::encryption::KeyPair::new();
    let arced_sk = Arc::new(dummy_keypair.private_key().to_owned());
    let dummy_mix_packet_processor = PacketProcessor::new(
        Arc::clone(&arced_sk),
        dummy_clients_handler_tx.clone(),
        client_storage,
    );

    ClientsHandler::new(dummy_clients_handler_rx, arced_sk).start();
    websocket::Listener::new(addr, dummy_clients_handler_tx.clone()).start();
    mixnet_handling::Listener::new(addr).start(dummy_mix_packet_processor);

    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }

    println!(
        "Received SIGINT - the provider will terminate now (threads are not YET nicely stopped)"
    );
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
