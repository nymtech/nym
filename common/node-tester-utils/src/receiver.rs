// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use futures::channel::mpsc;
use futures::StreamExt;
use nym_crypto::asymmetric::encryption;
use nym_sphinx::message::NymMessage;
use nym_sphinx::receiver::{MessageReceiver, SphinxMessageReceiver};
use nym_sphinx::{
    acknowledgements::{identifier::recover_identifier, AckKey},
    chunking::fragment::FragmentIdentifier,
};
use nym_task::TaskClient;
use std::sync::Arc;

pub type ReceivedSender = mpsc::UnboundedSender<Received>;
pub type ReceivedReceiver = mpsc::UnboundedReceiver<Received>;

// simple enum containing aggregated processed results
pub enum Received {
    Message(NymMessage),
    Ack(FragmentIdentifier),
}

impl From<NymMessage> for Received {
    fn from(value: NymMessage) -> Self {
        Received::Message(value)
    }
}

impl From<FragmentIdentifier> for Received {
    fn from(value: FragmentIdentifier) -> Self {
        Received::Ack(value)
    }
}

// the 'Simple' bit comes from the fact that it expects all received messages to consist of a single `Fragment`
pub struct SimpleMessageReceiver<R: MessageReceiver = SphinxMessageReceiver> {
    local_encryption_keypair: Arc<encryption::KeyPair>,

    ack_key: Arc<AckKey>,

    /// Structure responsible for decrypting and recovering plaintext message from received ciphertexts.
    message_receiver: R,

    mixnet_message_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
    acks_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,

    received_sender: ReceivedSender,
    shutdown: TaskClient,
}

impl SimpleMessageReceiver<SphinxMessageReceiver> {
    pub fn new_sphinx_receiver(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        ack_key: Arc<AckKey>,
        mixnet_message_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        acks_receiver: mpsc::UnboundedReceiver<Vec<Vec<u8>>>,
        received_sender: ReceivedSender,
        shutdown: TaskClient,
    ) -> Self {
        SimpleMessageReceiver {
            local_encryption_keypair,
            ack_key,
            message_receiver: SphinxMessageReceiver::new(),
            mixnet_message_receiver,
            acks_receiver,
            received_sender,shutdown
        }
    }
}

impl<R: MessageReceiver> SimpleMessageReceiver<R> {
    fn forward_received<T: Into<Received>>(&self, received: T) {
        // TODO: remove the unwrap once/if we do graceful shutdowns here
        self.received_sender
            .unbounded_send(received.into())
            .expect("ReceivedReceiver has stopped receiving");
    }

    fn on_mixnet_message(&mut self, mut raw_message: Vec<u8>) -> Result<(), NetworkTestingError> {
        let plaintext = self
            .message_receiver
            .recover_plaintext_from_regular_packet(
                self.local_encryption_keypair.private_key(),
                &mut raw_message,
            )?;
        let fragment = self.message_receiver.recover_fragment(plaintext)?;
        let (recovered, _) = self
            .message_receiver
            .insert_new_fragment(fragment)?
            .ok_or(NetworkTestingError::NonReconstructablePacket)?; // by definition of this receiver, the message must consist of a single fragment

        self.forward_received(recovered);
        Ok(())
    }

    fn on_ack(&mut self, raw_ack: Vec<u8>) -> Result<(), NetworkTestingError> {
        let serialized_ack = recover_identifier(&self.ack_key, &raw_ack)
            .ok_or(NetworkTestingError::UnrecoverableAck)?;

        let frag_id = FragmentIdentifier::try_from_bytes(serialized_ack)
            .map_err(|source| NetworkTestingError::MalformedAckIdentifier { source })?;

        self.forward_received(frag_id);
        Ok(())
    }

    // fn clear_channels(&mut self) {
    //     while self.mixnet_message_receiver.try_next().is_ok() {}
    //     while self.acks_receiver.try_next().is_ok() {}
    // }

    pub async fn run(&mut self) {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    todo!()
                }
                mixnet_messages = self.mixnet_message_receiver.next() => {
                    let Some(mixnet_messages) = mixnet_messages else {
                        todo!()
                    };
                    for message in mixnet_messages {
                        if let Err(err) = self.on_mixnet_message(message) {
                            todo!()
                        }
                    }
                }
                acks = self.acks_receiver.next() => {
                    let Some(acks) = acks else {
                        todo!()
                    };
                    for ack in acks {
                        if let Err(err) = self.on_ack(ack) {
                            todo!()
                        }
                    }
                }
            }
        }
    }
}
