// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::gateways_reader::GatewayMessages;
use crate::network_monitor::test_packet::{NodeTestMessage, NymApiTestMessageExt};
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use nym_crypto::asymmetric::encryption;
use nym_node_tester_utils::error::NetworkTestingError;
use nym_node_tester_utils::processor::TestPacketProcessor;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::receiver::{MessageReceiver, MessageRecoveryError};
use std::mem;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, trace, warn};

pub(crate) type ReceivedProcessorSender = mpsc::UnboundedSender<GatewayMessages>;
pub(crate) type ReceivedProcessorReceiver = mpsc::UnboundedReceiver<GatewayMessages>;

#[derive(Error, Debug)]
enum ProcessingError {
    #[error(
        "could not recover underlying data from the received packet since it was malformed: {0}"
    )]
    MalformedPacketReceived(#[from] MessageRecoveryError),

    #[error("the received test packet was malformed: {0}")]
    MalformedTestPacket(#[from] NetworkTestingError),

    #[error("received packet with an unexpected nonce. Got: {received}, expected: {expected}")]
    NonMatchingNonce { received: u64, expected: u64 },

    #[error("received a mix packet while no test run is currently in progress")]
    ReceivedOutsideTestRun,
}

#[derive(Clone)]
struct SharedProcessorData {
    inner: Arc<SharedProcessorDataInner>,
}

impl SharedProcessorData {
    async fn reset_run_information(&self) -> Vec<NodeTestMessage> {
        self.test_nonce.store(u64::MAX, Ordering::SeqCst);
        let mut guard = self.received_packets.lock().await;
        mem::take(&mut *guard)
    }
}

impl Deref for SharedProcessorData {
    type Target = SharedProcessorDataInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

struct SharedProcessorDataInner {
    /// Nonce of the current test run indicating which packets should get rejected.
    test_nonce: AtomicU64,

    /// Vector containing all received (and decrypted) packets in the current test run.
    // TODO: perhaps a different structure would be better here
    received_packets: Mutex<Vec<NodeTestMessage>>,
}

struct ReceiverTask<R: MessageReceiver> {
    shared: SharedProcessorData,
    packets_receiver: ReceivedProcessorReceiver,
    test_processor: TestPacketProcessor<NymApiTestMessageExt, R>,
}

impl<R> ReceiverTask<R>
where
    R: MessageReceiver,
{
    async fn on_received_data(&mut self, raw_message: Vec<u8>) -> Result<(), ProcessingError> {
        // if the nonce is none it means the packet was received during the 'waiting' for the
        // next test run
        let test_nonce = self.shared.test_nonce.load(Ordering::SeqCst);
        if test_nonce == u64::MAX {
            return Err(ProcessingError::ReceivedOutsideTestRun);
        }

        let test_msg = self.test_processor.process_mixnet_message(raw_message)?;

        if test_msg.ext.test_nonce != test_nonce {
            return Err(ProcessingError::NonMatchingNonce {
                received: test_msg.ext.test_nonce,
                expected: test_nonce,
            });
        }

        self.shared.received_packets.lock().await.push(test_msg);
        Ok(())
    }

    fn on_received_ack(&mut self, raw_ack: Vec<u8>) -> Result<(), ProcessingError> {
        // if the nonce is none it means the packet was received during the 'waiting' for the
        // next test run
        let test_nonce = self.shared.test_nonce.load(Ordering::SeqCst);
        if test_nonce == u64::MAX {
            return Err(ProcessingError::ReceivedOutsideTestRun);
        }

        let frag_id = self.test_processor.process_ack(raw_ack)?;
        // TODO: hook it up at some point
        trace!("received a test ack with id {frag_id}. However, we're not going to do anything about it (just yet)");

        Ok(())
    }

    async fn on_received(&mut self, messages: GatewayMessages) {
        match messages {
            GatewayMessages::Data(data_msgs) => {
                for raw in data_msgs {
                    if let Err(err) = self.on_received_data(raw).await {
                        warn!(target: "Monitor", "failed to process received gateway message: {err}")
                    }
                }
            }
            GatewayMessages::Acks(acks) => {
                for raw in acks {
                    if let Err(err) = self.on_received_ack(raw) {
                        warn!(target: "Monitor", "failed to process received gateway ack: {err}")
                    }
                }
            }
        }
    }
}

pub struct ReceivedProcessor<R: MessageReceiver> {
    shared: SharedProcessorData,
    receiver_task: Option<ReceiverTask<R>>,
}

impl<R> ReceivedProcessor<R>
where
    R: MessageReceiver,
{
    pub(crate) fn new(
        packets_receiver: ReceivedProcessorReceiver,
        client_encryption_keypair: Arc<encryption::KeyPair>,
        ack_key: Arc<AckKey>,
    ) -> Self {
        let shared_data = SharedProcessorData {
            inner: Arc::new(SharedProcessorDataInner {
                test_nonce: AtomicU64::new(u64::MAX),
                received_packets: Default::default(),
            }),
        };

        ReceivedProcessor {
            shared: shared_data.clone(),
            receiver_task: Some(ReceiverTask {
                shared: shared_data,
                packets_receiver,
                test_processor: TestPacketProcessor::new(client_encryption_keypair, ack_key),
            }),
        }
    }

    pub(crate) fn start_receiving(&mut self)
    where
        R: Sync + Send + 'static,
    {
        let mut receiver_task = self
            .receiver_task
            .take()
            .expect("network monitor has already started the receiver task!");

        tokio::spawn(async move {
            while let Some(messages) = receiver_task.packets_receiver.next().await {
                receiver_task.on_received(messages).await
            }
        });
    }

    pub(super) fn set_route_test_nonce(&self) {
        self.set_new_test_nonce(ROUTE_TESTING_TEST_NONCE)
    }

    pub(super) fn set_new_test_nonce(&self, test_nonce: u64) {
        self.shared.test_nonce.store(test_nonce, Ordering::SeqCst);
    }

    pub(super) async fn return_received(&self) -> Vec<NodeTestMessage> {
        self.shared.reset_run_information().await
    }
}
