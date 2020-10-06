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
use crate::monitor::NOTIFIER_DELIVERY_TIMEOUT;
use crate::notifications::test_timeout::TestTimeout;
use crate::test_packet::TestPacket;
use crate::test_run::{RunInfo, TestRunUpdate, TestRunUpdateReceiver};
use crypto::asymmetric::encryption::KeyPair;
use directory_client::mixmining::MixStatus;
use futures::stream::FuturesUnordered;
use futures::try_join;
use futures::StreamExt;
use log::*;
use nymsphinx::receiver::MessageReceiver;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::Arc;

mod test_timeout;

#[derive(Debug)]
enum NotifierError {
    DirectoryError(String),
    MalformedPacketReceived,
    NonTestPacketReceived,
    UnexpectedTestPacketReceived(TestPacket),
}

pub(crate) struct Notifier {
    client_encryption_keypair: KeyPair,
    message_receiver: MessageReceiver,
    mixnet_receiver: MixnetReceiver,
    directory_client: Arc<directory_client::Client>,
    test_run_receiver: TestRunUpdateReceiver,
    test_run_nonce: u64,
    test_timeout: TestTimeout,

    expected_run_packets: HashSet<TestPacket>,
}

impl Notifier {
    pub(crate) fn new(
        mixnet_receiver: MixnetReceiver,
        client_encryption_keypair: KeyPair,
        directory_client: Arc<directory_client::Client>,
        test_run_receiver: TestRunUpdateReceiver,
    ) -> Notifier {
        let message_receiver = MessageReceiver::new();
        Notifier {
            client_encryption_keypair,
            message_receiver,
            mixnet_receiver,
            directory_client,
            test_run_receiver,
            test_run_nonce: 0,
            test_timeout: TestTimeout::new(),
            expected_run_packets: HashSet::new(),
        }
    }

    async fn on_run_start(&mut self, run_info: RunInfo) {
        if run_info.nonce != self.test_run_nonce + 1 {
            error!(
                "Received unexpected test run info! Got {}, expected: {}",
                self.test_run_nonce + 1,
                run_info.nonce
            );
            return;
        }

        // notify about malformed nodes:
        for malformed_mix in run_info.malformed_mixes {
            if let Err(err) = self.notify_down(malformed_mix.clone()).await {
                error!(
                    "Failed to notify directory about {} being malformed - {:?}",
                    malformed_mix, err
                )
            }
        }

        // store information about packets that are currently being sent
        for test_packet in run_info.test_packets {
            self.expected_run_packets.insert(test_packet);
        }

        // we already checked that nonce is incremented by one
        self.test_run_nonce += 1;
    }

    fn undelivered_summary(&self, undelivered: &HashSet<TestPacket>) {
        let total_undelivered = undelivered.len();
        if total_undelivered != 0 {
            info!(target: "summary", "There are {} undelivered packets!", total_undelivered);

            let mut undelivered_v4 = 0;
            let mut undelivered_v6 = 0;

            let mut down_nodes = HashMap::new();
            for undelivered_packet in undelivered {
                let entry = down_nodes
                    .entry(undelivered_packet.pub_key_string())
                    .or_insert((true, true));
                if undelivered_packet.ip_version().is_v4() {
                    entry.0 = false;
                    undelivered_v4 += 1;
                } else {
                    entry.1 = false;
                    undelivered_v6 += 1;
                }
            }

            info!(target: "summary", "{} undelivered packets were IpV4, {} were IpV6", undelivered_v4, undelivered_v6);

            let mut non_v4_nodes = 0;
            let mut non_v6_nodes = 0;
            let mut messed_up_nodes = 0;

            for (down_node, result) in down_nodes.into_iter() {
                let down_str = match result {
                    (true, false) => {
                        non_v6_nodes += 1;
                        "failed to route IpV6 packet"
                    }
                    (false, true) => {
                        non_v4_nodes += 1;
                        "failed to route IpV4 packet"
                    }
                    (false, false) => {
                        messed_up_nodes += 1;
                        "failed to route BOTH IpV4 AND IpV6 packet"
                    }
                    (true, true) => panic!("This result is impossible!"),
                };

                info!(target: "detailed summary", "{} {}", down_node, down_str);
            }

            info!(target: "summary", "{} nodes don't speak ipv4 (!), {} nodes don't speak ipv6 and {} nodes don't speak either", non_v4_nodes, non_v6_nodes, messed_up_nodes);
        } else {
            info!(target: "summary", "Everything is working perfectly!")
        }
    }

    async fn on_timeout(&mut self) {
        let undelivered = mem::replace(&mut self.expected_run_packets, HashSet::new());
        self.undelivered_summary(&undelivered);

        // if we have a lot of undelivered packets we don't want to perform all directory calls
        // synchronously. There will be bunch of IO waiting for the network packets to
        // actually go through. Therefore try to do it concurrently.
        let mut directory_futures = FuturesUnordered::new();

        for undelivered_packet in undelivered.into_iter() {
            let dir_client = Arc::clone(&self.directory_client);
            let mix_id = undelivered_packet.pub_key_string();
            let future = async move {
                if let Err(err) = Self::notify_validator_with_client(
                    &*dir_client,
                    undelivered_packet.into_down_mixstatus(),
                )
                .await
                {
                    error!(
                        "Failed to notify directory about {} being down - {:?}",
                        mix_id, err
                    )
                }
            };
            directory_futures.push(future);
        }

        while !directory_futures.is_empty() {
            directory_futures.next().await;
        }
    }

    fn on_sending_over(&mut self, nonce: u64) {
        assert_eq!(nonce, self.test_run_nonce);
        self.test_timeout.start(NOTIFIER_DELIVERY_TIMEOUT);
    }

    async fn on_test_run_update(&mut self, run_update: TestRunUpdate) {
        match run_update {
            TestRunUpdate::StartSending(run_info) => self.on_run_start(run_info).await,
            TestRunUpdate::DoneSending(nonce) => self.on_sending_over(nonce),
        }
    }

    async fn on_mix_messages(&mut self, messages: Vec<Vec<u8>>) {
        for message in messages {
            if let Err(err) = self.on_message(message).await {
                error!(target: "Mix receiver", "failed to process received mix packet - {:?}", err)
            }
        }
    }

    pub(crate) async fn run(&mut self) {
        debug!("Started MixnetListener");
        loop {
            tokio::select! {
                mix_messages = &mut self.mixnet_receiver.next() => {
                    self.on_mix_messages(mix_messages.expect("mix channel has failed!")).await;
                },
                run_update = &mut self.test_run_receiver.next() => {
                    self.on_test_run_update(run_update.expect("packet sender has died!")).await;
                }
                _ = &mut self.test_timeout => {
                    self.on_timeout().await;
                    self.test_timeout.clear();
                }
            }
        }
    }

    async fn on_message(&mut self, message: Vec<u8>) -> Result<(), NotifierError> {
        let encrypted_bytes = self
            .message_receiver
            .recover_plaintext(self.client_encryption_keypair.private_key(), message)
            .map_err(|_| NotifierError::MalformedPacketReceived)?;
        let fragment = self
            .message_receiver
            .recover_fragment(&encrypted_bytes)
            .map_err(|_| NotifierError::MalformedPacketReceived)?;
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)
            .map_err(|_| NotifierError::MalformedPacketReceived)?
            .ok_or_else(|| NotifierError::NonTestPacketReceived)?; // if it's a test packet it MUST BE reconstructed with single fragment

        let test_packet = TestPacket::try_from_bytes(&recovered.message)
            .map_err(|_| NotifierError::NonTestPacketReceived)?;

        if self.expected_run_packets.remove(&test_packet) {
            self.notify_validator(test_packet.into_up_mixstatus())
                .await?;

            if self.expected_run_packets.is_empty() {
                self.test_timeout.fire();
            }

            Ok(())
        } else {
            Err(NotifierError::UnexpectedTestPacketReceived(test_packet))
        }
    }

    async fn notify_down(&self, pub_key: String) -> Result<(), NotifierError> {
        let v4_status = MixStatus {
            pub_key: pub_key.clone(),
            ip_version: "4".to_string(),
            up: false,
        };

        let v6_status = MixStatus {
            pub_key,
            ip_version: "6".to_string(),
            up: false,
        };

        let v4_future = self.notify_validator(v4_status);
        let v6_future = self.notify_validator(v6_status);

        try_join!(v4_future, v6_future)?;
        Ok(())
    }

    async fn notify_validator(&self, status: MixStatus) -> Result<(), NotifierError> {
        debug!("Sending status: {:?}", status);
        // self.directory_client
        //     .post_mixmining_status(status)
        //     .await
        //     .map_err(|err| NotifierError::DirectoryError(err.to_string()))?;
        Ok(())
    }

    async fn notify_validator_with_client(
        client: &directory_client::Client,
        status: MixStatus,
    ) -> Result<(), NotifierError> {
        debug!("Sending status: {:?}", status);
        // client
        //     .post_mixmining_status(status)
        //     .await
        //     .map_err(|err| NotifierError::DirectoryError(err.to_string()))?;
        Ok(())
    }
}
