// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::gateways_reader::GatewayMessages;
use crate::network_monitor::test_packet::{TestPacket, TestPacketError};
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use futures::channel::mpsc;
use futures::lock::{Mutex, MutexGuard};
use futures::{SinkExt, StreamExt};
use log::warn;
use nym_crypto::asymmetric::encryption;
use nym_sphinx::receiver::{MessageReceiver, MessageRecoveryError};
use std::mem;
use std::sync::Arc;
use thiserror::Error;

pub(crate) type ReceivedProcessorSender = mpsc::UnboundedSender<GatewayMessages>;
pub(crate) type ReceivedProcessorReceiver = mpsc::UnboundedReceiver<GatewayMessages>;

#[derive(Error, Debug)]
enum ProcessingError {
    #[error(
        "could not recover underlying data from the received packet since it was malformed - {0}"
    )]
    MalformedPacketReceived(#[from] MessageRecoveryError),

    #[error("received a mix packet that was NOT a proper network monitor test packet")]
    NonTestPacketReceived,

    #[error("the received test packet was malformed - {0}")]
    MalformedTestPacket(#[from] TestPacketError),

    #[error("received packet with an unexpected nonce. Got: {received}, expected: {expected}")]
    NonMatchingNonce { received: u64, expected: u64 },

    #[error("received a mix packet while no test run is currently in progress")]
    ReceivedOutsideTestRun,
}

// we can't use Notify due to possible edge case where both notification are consumed at once
enum LockPermit {
    Release,
    Free,
}

struct ReceivedProcessorInner {
    /// Nonce of the current test run indicating which packets should get rejected.
    test_nonce: Option<u64>,

    /// Channel for receiving packets/messages from the gateway clients
    packets_receiver: ReceivedProcessorReceiver,

    // TODO: right now it's identical for each gateway we send through, but should it?
    /// Encryption key of the clients sending through the gateways.
    client_encryption_keypair: Arc<encryption::KeyPair>,

    /// Structure responsible for decrypting and recovering plaintext message from received ciphertexts.
    message_receiver: MessageReceiver,

    /// Vector containing all received (and decrypted) packets in the current test run.
    received_packets: Vec<TestPacket>,
}

impl ReceivedProcessorInner {
    fn on_message(&mut self, mut message: Vec<u8>) -> Result<(), ProcessingError> {
        // if the nonce is none it means the packet was received during the 'waiting' for the
        // next test run
        if self.test_nonce.is_none() {
            return Err(ProcessingError::ReceivedOutsideTestRun);
        }

        let plaintext = self
            .message_receiver
            .recover_plaintext_from_regular_packet(
                self.client_encryption_keypair.private_key(),
                &mut message,
            )?;
        let fragment = self.message_receiver.recover_fragment(plaintext)?;
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)?
            .ok_or(ProcessingError::NonTestPacketReceived)?; // if it's a test packet it MUST BE reconstructed with single fragment
        let test_packet = TestPacket::try_from_bytes(&recovered.into_inner_data())?;

        // we know nonce is NOT none
        if test_packet.test_nonce() != self.test_nonce.unwrap() {
            return Err(ProcessingError::NonMatchingNonce {
                received: test_packet.test_nonce(),
                expected: self.test_nonce.unwrap(),
            });
        }

        self.received_packets.push(test_packet);

        Ok(())
    }

    fn finish_run(&mut self) -> Vec<TestPacket> {
        self.test_nonce = None;
        mem::take(&mut self.received_packets)
    }
}

pub(crate) struct ReceivedProcessor {
    permit_changer: Option<mpsc::Sender<LockPermit>>,
    inner: Arc<Mutex<ReceivedProcessorInner>>,
}

impl ReceivedProcessor {
    pub(crate) fn new(
        packets_receiver: ReceivedProcessorReceiver,
        client_encryption_keypair: Arc<encryption::KeyPair>,
    ) -> Self {
        let inner: Arc<Mutex<ReceivedProcessorInner>> =
            Arc::new(Mutex::new(ReceivedProcessorInner {
                test_nonce: None,
                packets_receiver,
                client_encryption_keypair,
                message_receiver: MessageReceiver::new(),
                received_packets: Vec::new(),
            }));

        ReceivedProcessor {
            permit_changer: None,
            inner,
        }
    }

    pub(crate) fn start_receiving(&mut self) {
        let inner = Arc::clone(&self.inner);

        // TODO: perhaps it should be using 0 size instead?
        let (permit_sender, mut permit_receiver) = mpsc::channel(1);
        self.permit_changer = Some(permit_sender);

        tokio::spawn(async move {
            while let Some(permit) = wait_for_permit(&mut permit_receiver, &inner).await {
                receive_or_release_permit(&mut permit_receiver, permit).await;
            }

            async fn receive_or_release_permit(
                permit_receiver: &mut mpsc::Receiver<LockPermit>,
                mut inner: MutexGuard<'_, ReceivedProcessorInner>,
            ) {
                loop {
                    tokio::select! {
                        permit_receiver = permit_receiver.next() => match permit_receiver {
                            Some(LockPermit::Release) => return,
                            Some(LockPermit::Free) => error!("somehow we got notification that the lock is free to take while we already hold it!"),
                            None => return,
                        },
                        messages = inner.packets_receiver.next() => match messages {
                            Some(messages) => {
                                for message in messages {
                                    if let Err(err) = inner.on_message(message) {
                                        warn!(target: "Monitor", "failed to process received gateway message - {err}")
                                    }
                                }
                            }
                            None => return,
                        },
                    }
                }
            }

            // // this lint really looks like a false positive because when lifetimes are elided,
            // // the compiler can't figure out appropriate lifetime bounds
            // #[allow(clippy::needless_lifetimes)]
            async fn wait_for_permit<'a: 'b, 'b>(
                permit_receiver: &'b mut mpsc::Receiver<LockPermit>,
                inner: &'a Mutex<ReceivedProcessorInner>,
            ) -> Option<MutexGuard<'a, ReceivedProcessorInner>> {
                loop {
                    match permit_receiver.next().await {
                        // we should only ever get this on the very first run
                        Some(LockPermit::Release) => debug!(
                            "somehow got request to drop our lock permit while we do not hold it!"
                        ),
                        Some(LockPermit::Free) => return Some(inner.lock().await),
                        None => return None,
                    }
                }
            }
        });
    }

    pub(super) async fn set_route_test_nonce(&mut self) {
        self.set_new_test_nonce(ROUTE_TESTING_TEST_NONCE).await
    }

    pub(super) async fn set_new_test_nonce(&mut self, test_nonce: u64) {
        // ask for the lock back
        self.permit_changer
            .as_mut()
            .expect("ReceivedProcessor hasn't started receiving!")
            .send(LockPermit::Release)
            .await
            .expect("processing task has died!");
        let mut inner = self.inner.lock().await;

        inner.test_nonce = Some(test_nonce);

        // give the permit back
        drop(inner);
        self.permit_changer
            .as_mut()
            .expect("ReceivedProcessor hasn't started receiving!")
            .send(LockPermit::Free)
            .await
            .expect("processing task has died!");
    }

    pub(super) async fn return_received(&mut self) -> Vec<TestPacket> {
        // ask for the lock back
        self.permit_changer
            .as_mut()
            .expect("ReceivedProcessor hasn't started receiving!")
            .send(LockPermit::Release)
            .await
            .expect("processing task has died!");
        let mut inner = self.inner.lock().await;

        let received = inner.finish_run();

        // give the permit back
        drop(inner);
        self.permit_changer
            .as_mut()
            .expect("ReceivedProcessor hasn't started receiving!")
            .send(LockPermit::Free)
            .await
            .expect("processing task has died!");

        received
    }
}
