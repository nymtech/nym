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

use config::NymConfig;
use std::time;
use tokio::runtime::Runtime;

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

fn main() {
    // totally optional
    setup_logging();

    let mut rt = Runtime::new().unwrap();

    // note: I have manually initialised the client with this config (because the keys weren't
    // saved, etc - a really dodgy setup but good enough for a quick test)
    // so to run it, you need to do the usual `nym-client init --id native-local`
    // then optionally modify config.toml to increase rates, etc. if you want
    let config = nym_client::config::Config::load_from_file(None, Some("native-local")).unwrap();
    let mut client = nym_client::client::NymClient::new(config);

    let input_data = std::fs::read("trailer.mp4").unwrap();

    client.start();
    let address = client.as_mix_destination();
    client.send_message(address, input_data);

    loop {
        let mut messages = rt.block_on(async {
            tokio::time::delay_for(time::Duration::from_secs(2)).await;
            client.check_for_messages_async().await
        });

        if !messages.is_empty() {
            assert_eq!(messages.len(), 1);
            std::fs::write("downloaded.mp4", messages.pop().unwrap()).unwrap();
            println!("done!");
            std::process::exit(0);
        }
    }
}
