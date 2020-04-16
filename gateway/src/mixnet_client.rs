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

// use log::*;
use futures::lock::Mutex;
use multi_tcp_client::Client as MultiClient;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::addressing::nodes::NODE_ADDRESS_LENGTH;
use std::sync::Arc;
use std::time::Duration;

pub fn new() -> Arc<Mutex<multi_tcp_client::Client>> {
    let config = multi_tcp_client::Config::new(
        Duration::from_millis(200),
        Duration::from_secs(86400),
        Duration::from_secs(2),
    );
    let client = multi_tcp_client::Client::new(config);
    Arc::new(Mutex::new(client))
}

pub async fn forward_to_mixnode(mut payload: Vec<u8>, client_ref: Arc<Mutex<MultiClient>>) {
    let mut address_buffer = [0; NODE_ADDRESS_LENGTH];
    let packet = payload.split_off(NODE_ADDRESS_LENGTH);
    address_buffer.copy_from_slice(payload.as_slice());
    let address = NymNodeRoutingAddress::try_from_bytes(&address_buffer)
        .unwrap()
        .into();

    let mut client = client_ref.lock().await;
    client.send(address, packet, false).await.unwrap();
}
