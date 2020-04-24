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
use crate::mixnet_handling::sender::PacketForwarder;
use crate::storage::ClientStorage;
use log::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

mod client_handling;
mod mixnet_handling;
pub(crate) mod storage;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    setup_logging();
    // TODO: assume config is parsed here, keys are loaded, etc
    // ALL OF BELOW WILL BE DONE VIA CONFIG
    let keypair = crypto::encryption::KeyPair::new();
    let clients_addr = "127.0.0.1:9000".parse().unwrap();
    let mix_addr = "127.0.0.1:1789".parse().unwrap();
    let inbox_store_dir: PathBuf = "foomp".into();
    let ledger_path: PathBuf = "foomp2".into();
    let message_retrieval_limit = 1000;
    let filename_len = 16;
    let initial_reconnection_backoff = Duration::from_millis(10_000);
    let maximum_reconnection_backoff = Duration::from_millis(300_000);
    let initial_connection_timeout = Duration::from_millis(1500);
    // ALL OF ABOVE WILL HAVE BEEN DONE VIA CONFIG

    let arced_sk = Arc::new(keypair.private_key().to_owned());

    // TODO: this should really be a proper DB, right now it will be most likely a bottleneck,
    // due to possible frequent independent writes
    let client_storage = ClientStorage::new(message_retrieval_limit, filename_len, inbox_store_dir);

    let (_, forwarding_channel) = PacketForwarder::new(
        initial_reconnection_backoff,
        maximum_reconnection_backoff,
        initial_connection_timeout,
    )
    .start();

    let (_, clients_handler_sender) =
        ClientsHandler::new(Arc::clone(&arced_sk), ledger_path, client_storage.clone()).start();

    let packet_processor =
        PacketProcessor::new(arced_sk, clients_handler_sender.clone(), client_storage);

    websocket::Listener::new(clients_addr).start(clients_handler_sender, forwarding_channel);
    mixnet_handling::Listener::new(mix_addr).start(packet_processor);

    info!("All up and running!");

    if let Err(e) = tokio::signal::ctrl_c().await {
        error!(
            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
            e
        );
    }

    println!(
        "Received SIGINT - the gateway will terminate now (threads are not YET nicely stopped)"
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
