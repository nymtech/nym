// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkTestingError;
use crate::TestMessage;
use nym_crypto::asymmetric::encryption;
use nym_sphinx::acknowledgements::identifier::recover_identifier;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::chunking::fragment::FragmentIdentifier;
use nym_sphinx::receiver::{MessageReceiver, SphinxMessageReceiver};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::sync::Arc;

// simple enum containing aggregated processed results
pub enum Received<T> {
    Message(TestMessage<T>),
    Ack(FragmentIdentifier),
}

impl<T> From<TestMessage<T>> for Received<T> {
    fn from(value: TestMessage<T>) -> Self {
        Received::Message(value)
    }
}

impl<T> From<FragmentIdentifier> for Received<T> {
    fn from(value: FragmentIdentifier) -> Self {
        Received::Ack(value)
    }
}

pub struct TestPacketProcessor<T, R: MessageReceiver = SphinxMessageReceiver> {
    local_encryption_keypair: Arc<encryption::KeyPair>,
    ack_key: Arc<AckKey>,

    /// Structure responsible for decrypting and recovering plaintext message from received ciphertexts.
    message_receiver: R,

    _ext_phantom: PhantomData<T>,
}

impl<T> TestPacketProcessor<T, SphinxMessageReceiver> {
    pub fn new_sphinx_processor(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        ack_key: Arc<AckKey>,
    ) -> Self {
        Self::new(local_encryption_keypair, ack_key)
    }
}

impl<T, R> TestPacketProcessor<T, R>
where
    R: MessageReceiver,
{
    pub fn new(local_encryption_keypair: Arc<encryption::KeyPair>, ack_key: Arc<AckKey>) -> Self {
        TestPacketProcessor {
            local_encryption_keypair,
            ack_key,
            message_receiver: R::new(),
            _ext_phantom: PhantomData,
        }
    }

    pub fn process_mixnet_message(
        &mut self,
        mut raw_message: Vec<u8>,
    ) -> Result<TestMessage<T>, NetworkTestingError>
    where
        T: DeserializeOwned,
    {
        println!("process_mixnet message");
        let plaintext = self
            .message_receiver
            .recover_plaintext_from_regular_packet(
                self.local_encryption_keypair.private_key(),
                &mut raw_message,
            )?;
        let fragment = self.message_receiver.recover_fragment(plaintext)?;

        // test messages must consist of a single fragment
        let (serialized, _) = self
            .message_receiver
            .insert_new_fragment(fragment)?
            .ok_or(NetworkTestingError::NonReconstructablePacket)?;

        TestMessage::try_recover(serialized)
    }

    pub fn process_ack(
        &mut self,
        raw_ack: Vec<u8>,
    ) -> Result<FragmentIdentifier, NetworkTestingError> {
        let serialized_ack = recover_identifier(&self.ack_key, &raw_ack)
            .ok_or(NetworkTestingError::UnrecoverableAck)?;

        FragmentIdentifier::try_from_bytes(serialized_ack)
            .map_err(|source| NetworkTestingError::MalformedAckIdentifier { source })
    }
}
