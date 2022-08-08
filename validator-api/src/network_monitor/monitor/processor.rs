// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::gateways_reader::GatewayMessages;
use crate::network_monitor::test_packet::TestPacket;
use crate::network_monitor::ROUTE_TESTING_TEST_NONCE;
use crypto::asymmetric::encryption;
use futures::channel::mpsc;
use futures::lock::{Mutex, MutexGuard};
use futures::{SinkExt, StreamExt};
use log::warn;
use nymsphinx::receiver::MessageReceiver;
use std::fmt::{self, Display, Formatter};
use std::mem;
use std::sync::Arc;

pub(crate) type ReceivedProcessorSender = mpsc::UnboundedSender<GatewayMessages>;
pub(crate) type ReceivedProcessorReceiver = mpsc::UnboundedReceiver<GatewayMessages>;

#[derive(Debug)]
enum ProcessingError {
    MalformedPacketReceived,
    NonTestPacketReceived,
    NonMatchingNonce(u64),
    ReceivedOutsideTestRun,
}

impl Display for ProcessingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingError::MalformedPacketReceived => write!(f, "received malformed packet"),
            ProcessingError::NonTestPacketReceived => write!(f, "received a non-test packet"),
            ProcessingError::NonMatchingNonce(nonce) => write!(
                f,
                "received packet with nonce {} which is different than the expected",
                nonce
            ),
            ProcessingError::ReceivedOutsideTestRun => write!(
                f,
                "received packet while the test is currently not in progress"
            ),
        }
    }
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
    fn on_message(&mut self, message: Vec<u8>) -> Result<(), ProcessingError> {
        // if the nonce is none it means the packet was received during the 'waiting' for the
        // next test run
        if self.test_nonce.is_none() {
            return Err(ProcessingError::ReceivedOutsideTestRun);
        }

        let encrypted_bytes = self
            .message_receiver
            .recover_plaintext(self.client_encryption_keypair.private_key(), message)
            .map_err(|_| ProcessingError::MalformedPacketReceived)?;
        let fragment = self
            .message_receiver
            .recover_fragment(&encrypted_bytes)
            .map_err(|_| ProcessingError::MalformedPacketReceived)?;
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)
            .map_err(|_| ProcessingError::MalformedPacketReceived)?
            .ok_or(ProcessingError::NonTestPacketReceived)?; // if it's a test packet it MUST BE reconstructed with single fragment
        let test_packet = TestPacket::try_from_bytes(&recovered.message)
            .map_err(|_| ProcessingError::MalformedPacketReceived)?;

        // we know nonce is NOT none
        if test_packet.test_nonce() != self.test_nonce.unwrap() {
            return Err(ProcessingError::NonMatchingNonce(test_packet.test_nonce()));
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
            loop {
                let permit = wait_for_permit(&mut permit_receiver, &*inner).await;
                receive_or_release_permit(&mut permit_receiver, permit).await;
            }

            async fn receive_or_release_permit(
                permit_receiver: &mut mpsc::Receiver<LockPermit>,
                mut inner: MutexGuard<'_, ReceivedProcessorInner>,
            ) {
                loop {
                    tokio::select! {
                        permit_receiver = permit_receiver.next() => match permit_receiver.unwrap() {
                            LockPermit::Release => return,
                            LockPermit::Free => error!("somehow we got notification that the lock is free to take while we already hold it!"),
                        },
                        messages = inner.packets_receiver.next() => {
                            for message in messages.expect("packet receiver has died!") {
                                if let Err(err) = inner.on_message(message) {
                                    warn!(target: "Monitor", "failed to process received gateway message - {}", err)
                                }
                            }
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
            ) -> MutexGuard<'a, ReceivedProcessorInner> {
                loop {
                    match permit_receiver.next().await.unwrap() {
                        // we should only ever get this on the very first run
                        LockPermit::Release => debug!(
                            "somehow got request to drop our lock permit while we do not hold it!"
                        ),
                        LockPermit::Free => return inner.lock().await,
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
