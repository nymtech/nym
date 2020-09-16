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

use crypto::asymmetric::encryption;
use crypto::shared_key::recompute_shared_key;
use crypto::symmetric::stream_cipher;
use nymsphinx_anonymous_replies::reply_surb::{ReplySURB, ReplySURBError};
use nymsphinx_chunking::fragment::Fragment;
use nymsphinx_chunking::reconstruction::MessageReconstructor;
use nymsphinx_params::{PacketEncryptionAlgorithm, PacketHkdfAlgorithm, DEFAULT_NUM_MIX_HOPS};

// TODO: should this live in this file?
#[derive(Debug)]
pub struct ReconstructedMessage {
    /// The actual plaintext message that was received.
    pub message: Vec<u8>,

    /// Optional ReplySURB to allow for an anonymous reply to the sender.
    pub reply_surb: Option<ReplySURB>,
}

#[derive(Debug)]
pub enum MessageRecoveryError {
    InvalidSurbPrefixError,
    MalformedSURBError(ReplySURBError),
    InvalidRemoteEphemeralKey(encryption::EncryptionKeyError),
    MalformedFragmentError,
    InvalidMessagePaddingError,
    MalformedReconstructedMessage(Vec<i32>),
    TooShortMessageError,
}

impl From<ReplySURBError> for MessageRecoveryError {
    fn from(err: ReplySURBError) -> Self {
        MessageRecoveryError::MalformedSURBError(err)
    }
}

impl From<encryption::EncryptionKeyError> for MessageRecoveryError {
    fn from(err: encryption::EncryptionKeyError) -> Self {
        MessageRecoveryError::InvalidRemoteEphemeralKey(err)
    }
}

pub struct MessageReceiver {
    /// High level public structure used to buffer all received data [`Fragment`]s and eventually
    /// returning original messages that they encapsulate.
    reconstructor: MessageReconstructor,

    /// Number of mix hops each packet ('real' message, ack, reply) is expected to take.
    /// Note that it does not include gateway hops.
    num_mix_hops: u8,
}

impl MessageReceiver {
    pub fn new() -> Self {
        Default::default()
    }

    /// Allows setting non-default number of expected mix hops in the network.
    pub fn with_mix_hops(mut self, hops: u8) -> Self {
        self.num_mix_hops = hops;
        self
    }

    /// Parses the message to strip and optionally recover reply SURB.
    fn recover_reply_surb_from_message(
        &self,
        message: &mut Vec<u8>,
    ) -> Result<Option<ReplySURB>, MessageRecoveryError> {
        match message[0] {
            n if n == false as u8 => {
                message.remove(0);
                Ok(None)
            }
            n if n == true as u8 => {
                let surb_len: usize = ReplySURB::serialized_len(self.num_mix_hops);
                // note the extra +1 (due to 0/1 message prefix)
                let surb_bytes = &message[1..1 + surb_len];
                let reply_surb = ReplySURB::from_bytes(surb_bytes)?;

                *message = message.drain(1 + surb_len..).collect();
                Ok(Some(reply_surb))
            }
            _ => Err(MessageRecoveryError::InvalidSurbPrefixError),
        }
    }

    /// Given raw fragment data, recovers the remote ephemeral key, recomputes shared secret,
    /// uses it to decrypt fragment data
    pub fn recover_plaintext(
        &self,
        local_key: &encryption::PrivateKey,
        mut raw_enc_frag: Vec<u8>,
    ) -> Result<Vec<u8>, MessageRecoveryError> {
        // 1. recover remote encryption key
        let remote_key_bytes = &raw_enc_frag[..encryption::PUBLIC_KEY_SIZE];
        let remote_ephemeral_key = encryption::PublicKey::from_bytes(remote_key_bytes)?;

        // 2. recompute shared encryption key
        let encryption_key = recompute_shared_key::<PacketEncryptionAlgorithm, PacketHkdfAlgorithm>(
            &remote_ephemeral_key,
            local_key,
        );

        // 3. decrypt fragment data
        let fragment_bytes = &mut raw_enc_frag[encryption::PUBLIC_KEY_SIZE..];

        let zero_iv = stream_cipher::zero_iv::<PacketEncryptionAlgorithm>();
        Ok(stream_cipher::decrypt::<PacketEncryptionAlgorithm>(
            &encryption_key,
            &zero_iv,
            &fragment_bytes,
        ))
    }

    /// Given fragment data recovers [`Fragment`] itself.
    pub fn recover_fragment(&self, frag_data: &[u8]) -> Result<Fragment, MessageRecoveryError> {
        Fragment::try_from_bytes(frag_data)
            .map_err(|_| MessageRecoveryError::MalformedFragmentError)
    }

    /// Removes the zero padding from the message that was initially included to ensure same length
    /// sphinx payloads.
    pub fn remove_padding(message: &mut Vec<u8>) -> Result<(), MessageRecoveryError> {
        // we are looking for first occurrence of 1 in the tail and we get its index
        if let Some(i) = message.iter().rposition(|b| *b == 1) {
            // and now we only take bytes until that point (but not including it)
            *message = message.drain(..i).collect();
            Ok(())
        } else {
            Err(MessageRecoveryError::InvalidMessagePaddingError)
        }
    }

    /// Inserts given [`Fragment`] into the reconstructor.
    /// If it was last remaining [`Fragment`] for the original message, the message is reconstructed
    /// and returned alongside all (if applicable) set ids used in the message.
    ///
    /// # Returns:
    /// - The reconstructed message alongside optional reply SURB,
    /// - List of ids of all the [`Set`]s used during reconstruction to detect stale retransmissions.
    pub fn insert_new_fragment(
        &mut self,
        fragment: Fragment,
    ) -> Result<Option<(ReconstructedMessage, Vec<i32>)>, MessageRecoveryError> {
        if let Some((mut message, used_sets)) = self.reconstructor.insert_new_fragment(fragment) {
            // Split message into plaintext and reply-SURB
            let reply_surb = match self.recover_reply_surb_from_message(&mut message) {
                Ok(reply_surb) => reply_surb,
                Err(_) => {
                    return Err(MessageRecoveryError::MalformedReconstructedMessage(
                        used_sets,
                    ));
                }
            };

            // Finally, remove the zero padding from the message
            if Self::remove_padding(&mut message).is_err() {
                return Err(MessageRecoveryError::MalformedReconstructedMessage(
                    used_sets,
                ));
            };

            Ok(Some((
                ReconstructedMessage {
                    message,
                    reply_surb,
                },
                used_sets,
            )))
        } else {
            Ok(None)
        }
    }
}

impl Default for MessageReceiver {
    fn default() -> Self {
        MessageReceiver {
            reconstructor: Default::default(),
            num_mix_hops: DEFAULT_NUM_MIX_HOPS,
        }
    }
}

#[cfg(test)]
mod message_receiver {
    use super::*;
    use crypto::asymmetric::identity;
    use nymsphinx_addressing::clients::Recipient;
    use rand::rngs::OsRng;
    use std::collections::HashMap;
    use std::time::Duration;
    use topology::{gateway, mix, NymTopology};

    // TODO: is it somehow maybe possible to move it to `topology` and have if conditionally
    // available to other modules?
    /// Returns a hardcoded, valid instance of [`NymTopology`] that is to be used in
    /// tests requiring instance of topology.
    fn topology_fixture() -> NymTopology {
        let mut mixes = HashMap::new();
        mixes.insert(
            1,
            vec![mix::Node {
                location: "unknown".to_string(),
                host: "10.20.30.40:1789".parse().unwrap(),
                pub_key: encryption::PublicKey::from_base58_string(
                    "B3GzG62aXAZNg14RoMCp3BhELNBrySLr2JqrwyfYFzRc",
                )
                .unwrap(),
                layer: 1,
                last_seen: 1594812897745695000,
                version: "0.8.0-dev".to_string(),
            }],
        );

        mixes.insert(
            2,
            vec![mix::Node {
                location: "unknown".to_string(),
                host: "11.21.31.41:1789".parse().unwrap(),
                pub_key: encryption::PublicKey::from_base58_string(
                    "5Z1VqYwM2xeKxd8H7fJpGWasNiDFijYBAee7MErkZ5QT",
                )
                .unwrap(),
                layer: 2,
                last_seen: 1594812897745695000,
                version: "0.8.0-dev".to_string(),
            }],
        );

        mixes.insert(
            3,
            vec![mix::Node {
                location: "unknown".to_string(),
                host: "12.22.32.42:1789".parse().unwrap(),
                pub_key: encryption::PublicKey::from_base58_string(
                    "9EyjhCggr2QEA2nakR88YHmXgpy92DWxoe2draDRkYof",
                )
                .unwrap(),
                layer: 3,
                last_seen: 1594812897745695000,
                version: "0.8.0-dev".to_string(),
            }],
        );

        NymTopology::new(
            // currently coco_nodes don't really exist so this is still to be determined
            vec![],
            mixes,
            vec![gateway::Node {
                location: "unknown".to_string(),
                client_listener: "ws://1.2.3.4:9000".to_string(),
                mixnet_listener: "1.2.3.4:1789".parse().unwrap(),
                identity_key: identity::PublicKey::from_base58_string(
                    "FioFa8nMmPpQnYi7JyojoTuwGLeyNS8BF4ChPr29zUML",
                )
                .unwrap(),
                sphinx_key: encryption::PublicKey::from_base58_string(
                    "EB42xvMFMD5rUCstE2CDazgQQJ22zLv8SPm1Luxni44c",
                )
                .unwrap(),
                last_seen: 1594812897745695000,
                version: "0.8.0-dev".to_string(),
            }],
        )
    }

    #[test]
    fn correctly_splits_message_into_plaintext_and_surb() {
        let message_receiver: MessageReceiver = Default::default();

        // the actual 'correctness' of the underlying message doesn't matter for this test
        let message = vec![42; 100];
        let dummy_recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@FioFa8nMmPpQnYi7JyojoTuwGLeyNS8BF4ChPr29zUML").unwrap();
        let average_delay = Duration::from_millis(500);
        let topology = topology_fixture();

        let reply_surb =
            ReplySURB::construct(&mut OsRng, &dummy_recipient, average_delay, &topology).unwrap();

        let reply_surb_bytes = reply_surb.to_bytes();

        // this is not exactly what is 'received' but rather after "some" processing, however,
        // this is the expected argument to the function
        let mut received_without_surb: Vec<_> =
            std::iter::once(0).chain(message.iter().cloned()).collect();

        let reply_surb = message_receiver
            .recover_reply_surb_from_message(&mut received_without_surb)
            .unwrap();
        assert_eq!(received_without_surb, message);
        assert!(reply_surb.is_none());

        let mut received_with_surb: Vec<_> = std::iter::once(1)
            .chain(reply_surb_bytes.iter().cloned())
            .chain(message.iter().cloned())
            .collect();
        let reply_surb = message_receiver
            .recover_reply_surb_from_message(&mut received_with_surb)
            .unwrap();
        assert_eq!(received_with_surb, message);
        assert_eq!(reply_surb_bytes, reply_surb.unwrap().to_bytes());
    }
}
