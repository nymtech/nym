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

use super::monitor::MixnetReceiver;
use crypto::asymmetric::encryption::KeyPair;
use directory_client::mixmining::MixStatus;
use futures::StreamExt;
use log::debug;
use nymsphinx::receiver::MessageReceiver;
use std::sync::Arc;

pub(crate) struct Notifier {
    client_encryption_keypair: KeyPair,
    message_receiver: MessageReceiver,
    mixnet_receiver: MixnetReceiver,
    directory_client: Arc<directory_client::Client>,
}

impl Notifier {
    pub(crate) fn new(
        mixnet_receiver: MixnetReceiver,
        client_encryption_keypair: KeyPair,
        directory_client: Arc<directory_client::Client>,
    ) -> Notifier {
        let message_receiver = MessageReceiver::new();
        Notifier {
            client_encryption_keypair,
            message_receiver,
            mixnet_receiver,
            directory_client,
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started MixnetListener");
        while let Some(messages) = self.mixnet_receiver.next().await {
            for message in messages {
                self.on_message(message).await;
            }
        }
        panic!("MixnetListener channel failed. This should never happen.")
    }

    async fn on_message(&mut self, message: Vec<u8>) {
        let encrypted_bytes = self
            .message_receiver
            .recover_plaintext(self.client_encryption_keypair.private_key(), message)
            .expect("could not recover plaintext from test packet");
        let fragment = self
            .message_receiver
            .recover_fragment(&encrypted_bytes)
            .expect("could not recover fragment from test packet");
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)
            .expect("error when reconstructing test packet message")
            .expect("no reconstructed message received from test packet");
        let message = String::from_utf8_lossy(&recovered.message);
        println!("got msg: {:?}", message);
        if message.contains(":") {
            let split: Vec<&str> = message.split(":").collect();
            let pub_key = split[0].to_string();
            let ip_version = split[1].to_string();
            let status = MixStatus {
                pub_key,
                ip_version,
                up: true,
            };
            self.notify_validator(status).await;
        }
    }

    async fn notify_validator(&self, status: MixStatus) {
        println!("Sending status: {:?}", status);
        self.directory_client
            .post_mixmining_status(status)
            .await
            .unwrap();
    }
}
