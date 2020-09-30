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
use directory_client::presence::mixnodes::MixNodePresence;
use directory_client::DirectoryClient;
use log::{error, trace};
use std::time::Duration;
use tokio::task::JoinHandle;

pub(crate) struct NotifierConfig {
    location: String,
    directory_server: String,
    announce_host: String,
    pub_key_string: String,
    layer: u64,
    sending_delay: Duration,
}

impl NotifierConfig {
    pub(crate) fn new(
        location: String,
        directory_server: String,
        announce_host: String,
        pub_key_string: String,
        layer: u64,
        sending_delay: Duration,
    ) -> Self {
        NotifierConfig {
            location,
            directory_server,
            announce_host,
            pub_key_string,
            layer,
            sending_delay,
        }
    }
}

pub(crate) struct Notifier {
    net_client: directory_client::Client,
    presence: MixNodePresence,
    sending_delay: Duration,
}

impl Notifier {
    pub(crate) fn new(config: NotifierConfig) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);
        let presence = MixNodePresence {
            location: config.location,
            host: config.announce_host,
            pub_key: config.pub_key_string,
            layer: config.layer,
            last_seen: 0,
            version: built_info::PKG_VERSION.to_string(),
        };
        Notifier {
            net_client,
            presence,
            sending_delay: config.sending_delay,
        }
    }

    async fn notify(&self) {
        match self
            .net_client
            .post_mixnode_presence(self.presence.clone())
            .await
        {
            Err(err) => error!("failed to send presence - {:?}", err),
            Ok(_) => trace!("sent presence information"),
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                // set the deadline in the future
                let sending_delay = tokio::time::delay_for(self.sending_delay);
                self.notify().await;
                // wait for however much is left
                sending_delay.await;
            }
        })
    }
}
