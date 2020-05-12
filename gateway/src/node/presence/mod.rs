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

use crate::built_info;
use crate::node::storage::ClientLedger;
use directory_client::presence::gateways::{GatewayClient, GatewayPresence};
use directory_client::DirectoryClient;
use log::{error, trace};
use std::time::Duration;
use tokio::task::JoinHandle;

pub(crate) struct NotifierConfig {
    location: String,
    directory_server: String,
    mix_announce_host: String,
    clients_announce_host: String,
    pub_key_string: String,
    sending_delay: Duration,
}

impl NotifierConfig {
    pub(crate) fn new(
        location: String,
        directory_server: String,
        mix_announce_host: String,
        clients_announce_host: String,
        pub_key_string: String,
        sending_delay: Duration,
    ) -> Self {
        NotifierConfig {
            location,
            directory_server,
            mix_announce_host,
            clients_announce_host,
            pub_key_string,
            sending_delay,
        }
    }
}

pub(crate) struct Notifier {
    location: String,
    net_client: directory_client::Client,
    client_ledger: ClientLedger,
    sending_delay: Duration,
    client_listener: String,
    mixnet_listener: String,
    pub_key_string: String,
}

impl Notifier {
    pub(crate) fn new(config: NotifierConfig, client_ledger: ClientLedger) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);

        Notifier {
            client_ledger,
            net_client,
            location: config.location,
            client_listener: config.clients_announce_host,
            mixnet_listener: config.mix_announce_host,
            pub_key_string: config.pub_key_string,
            sending_delay: config.sending_delay,
        }
    }

    async fn make_presence(&self) -> GatewayPresence {
        let client_keys = self.client_ledger.current_clients().unwrap();
        let registered_clients = client_keys
            .into_iter()
            .map(|key_bytes| GatewayClient {
                pub_key: key_bytes.to_base58_string(),
            })
            .collect();

        GatewayPresence {
            location: self.location.clone(),
            client_listener: self.client_listener.clone(),
            mixnet_listener: self.mixnet_listener.clone(),
            pub_key: self.pub_key_string.clone(),
            registered_clients,
            last_seen: 0,
            version: built_info::PKG_VERSION.to_string(),
        }
    }

    async fn notify(&self, presence: GatewayPresence) {
        match self.net_client.post_gateway_presence(presence).await {
            Err(err) => error!("failed to send presence - {:?}", err),
            Ok(_) => trace!("sent presence information"),
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                // set the deadline in the future
                let sending_delay = tokio::time::delay_for(self.sending_delay);
                let presence = self.make_presence().await;
                self.notify(presence).await;
                // wait for however much is left
                sending_delay.await;
            }
        })
    }
}
