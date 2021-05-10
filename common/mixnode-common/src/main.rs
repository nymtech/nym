// Copyright 2021 Nym Technologies SA
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

use crypto::asymmetric::identity;
use rand::thread_rng;
use std::sync::Arc;

pub mod rtt_measurement;

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
        .filter_module("sled", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .init();
}

/*

*/

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("{:?}", args);

    let priv1 =
        identity::PrivateKey::from_base58_string("7NoHs7J1oSfYYs2Br4YLsf5rLCuArwNhAC56RCNiFLRq")
            .unwrap();
    let pub1 =
        identity::PublicKey::from_base58_string("Be9wH7xuXBRJAuV1pC7MALZv6a61RvWQ3SypsNarqTt")
            .unwrap();

    let pair1 = identity::KeyPair::from_bytes(&priv1.to_bytes(), &pub1.to_bytes()).unwrap();

    let priv2 =
        identity::PrivateKey::from_base58_string("46f71CYZf8R9BTQmDFuhu83aEwu2vNpS9H72y36i6goc")
            .unwrap();
    let pub2 =
        identity::PublicKey::from_base58_string("8a2x7DH5c1R8vHSyGu6KFnMgK1CgW3gw7wX2TaLSWjQq")
            .unwrap();

    let pair2 = identity::KeyPair::from_bytes(&priv2.to_bytes(), &pub2.to_bytes()).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        setup_logging();
        let id = Arc::new(pair2);

        let foo = rtt_measurement::RttMeasurer::new("[::]:1790".parse().unwrap(), id);
        foo.run().await;
    });
}
