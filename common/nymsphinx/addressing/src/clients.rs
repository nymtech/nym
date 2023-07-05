// This is still not an ideal home for this struct, because it's not an
// universal nymsphinx addressing method, however, it needs to be
// accessible by both desktop and webassembly client (it's more
// of a helper/utils structure, because before it reaches the gateway
// it's already destructed).

use crate::nodes::{NodeIdentity, NODE_IDENTITY_SIZE};
use nym_crypto::asymmetric::{encryption, identity};
use nym_sphinx_types::Destination;
use serde::de::{Error as SerdeError, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Formatter};
use std::str::FromStr;
use thiserror::Error;

// Not entirely sure whether this is the correct place for those, but let's see how it's going
// to work out
pub type ClientEncryptionKey = encryption::PublicKey;
const CLIENT_ENCRYPTION_KEY_SIZE: usize = encryption::PUBLIC_KEY_SIZE;

pub type ClientIdentity = identity::PublicKey;
const CLIENT_IDENTITY_SIZE: usize = identity::PUBLIC_KEY_LENGTH;

pub type RecipientBytes = [u8; Recipient::LEN];

#[derive(Debug, Error)]
pub enum RecipientFormattingError {
    #[error("recipient is malformed - {reason} ")]
    MalformedRecipientError { reason: String },

    #[error("recipient's identity key is malformed: {0}")]
    MalformedIdentityError(identity::Ed25519RecoveryError),

    #[error("recipient's encryption key is malformed: {0}")]
    MalformedEncryptionKeyError(#[from] encryption::KeyRecoveryError),

    #[error("recipient gateway's identity key is malformed: {0}")]
    MalformedGatewayError(identity::Ed25519RecoveryError),
}

// TODO: this should a different home... somewhere, but where?
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Recipient {
    client_identity: ClientIdentity,
    client_encryption_key: ClientEncryptionKey,
    gateway: NodeIdentity,
}

// Serialize + Deserialize is not really used anymore (it was for a CBOR experiment)
// however, if we decided we needed it again, it's already here
impl Serialize for Recipient {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.to_bytes())
    }
}

impl<'de> Deserialize<'de> for Recipient {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct RecipientVisitor;

        impl<'de> Visitor<'de> for RecipientVisitor {
            type Value = Recipient;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, "A recipient is 96-byte sequence containing two ed25519 public keys and one x25519 public key all in compressed forms.")
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: SerdeError,
            {
                if bytes.len() != Recipient::LEN {
                    return Err(SerdeError::invalid_length(bytes.len(), &self));
                }

                let mut recipient_bytes = [0u8; Recipient::LEN];
                // this shouldn't panic as we just checked for length
                recipient_bytes.copy_from_slice(bytes);

                Recipient::try_from_bytes(recipient_bytes).map_err(|_| {
                    SerdeError::invalid_value(
                        Unexpected::Other("At least one of the curve points was malformed"),
                        &self,
                    )
                })
            }
        }

        deserializer.deserialize_bytes(RecipientVisitor)
    }
}

impl Recipient {
    pub const LEN: usize = CLIENT_IDENTITY_SIZE + CLIENT_ENCRYPTION_KEY_SIZE + NODE_IDENTITY_SIZE;

    pub fn new(
        client_identity: ClientIdentity,
        client_encryption_key: ClientEncryptionKey,
        gateway: NodeIdentity,
    ) -> Self {
        Recipient {
            client_identity,
            client_encryption_key,
            gateway,
        }
    }

    // TODO: Currently the `DestinationAddress` is equivalent to `ClientIdentity`, but perhaps
    // it shouldn't be? Maybe it should be (for example) H(`ClientIdentity || ClientEncryptionKey`)
    // instead? That is an open question.
    pub fn as_sphinx_destination(&self) -> Destination {
        // since the nym mix network differs slightly in design from loopix, we do not care
        // about "surb_id" field at all and just use the default value.
        Destination::new(
            self.client_identity.derive_destination_address(),
            Default::default(),
        )
    }

    pub fn identity(&self) -> &ClientIdentity {
        &self.client_identity
    }

    pub fn encryption_key(&self) -> &ClientEncryptionKey {
        &self.client_encryption_key
    }

    pub fn gateway(&self) -> &NodeIdentity {
        &self.gateway
    }

    pub fn to_bytes(self) -> RecipientBytes {
        let mut out = [0u8; Self::LEN];
        out[..CLIENT_IDENTITY_SIZE].copy_from_slice(&self.client_identity.to_bytes());
        out[CLIENT_IDENTITY_SIZE..CLIENT_IDENTITY_SIZE + CLIENT_ENCRYPTION_KEY_SIZE]
            .copy_from_slice(&self.client_encryption_key.to_bytes());
        out[CLIENT_IDENTITY_SIZE + CLIENT_ENCRYPTION_KEY_SIZE..]
            .copy_from_slice(&self.gateway.to_bytes());

        out
    }

    pub fn try_from_bytes(bytes: RecipientBytes) -> Result<Self, RecipientFormattingError> {
        let identity_bytes = &bytes[..CLIENT_IDENTITY_SIZE];
        let enc_key_bytes =
            &bytes[CLIENT_IDENTITY_SIZE..CLIENT_IDENTITY_SIZE + CLIENT_ENCRYPTION_KEY_SIZE];
        let gateway_bytes = &bytes[CLIENT_IDENTITY_SIZE + CLIENT_ENCRYPTION_KEY_SIZE..];

        let client_identity = match ClientIdentity::from_bytes(identity_bytes) {
            Ok(client_id) => client_id,
            Err(err) => return Err(RecipientFormattingError::MalformedIdentityError(err)),
        };

        let client_encryption_key = ClientEncryptionKey::from_bytes(enc_key_bytes)?;

        let gateway = match NodeIdentity::from_bytes(gateway_bytes) {
            Ok(gate_id) => gate_id,
            Err(err) => return Err(RecipientFormattingError::MalformedGatewayError(err)),
        };

        Ok(Recipient {
            client_identity,
            client_encryption_key,
            gateway,
        })
    }

    pub fn try_from_base58_string<S: Into<String>>(
        full_address: S,
    ) -> Result<Self, RecipientFormattingError> {
        let string_address = full_address.into();
        let split: Vec<_> = string_address.split('@').collect();
        if split.len() != 2 {
            return Err(RecipientFormattingError::MalformedRecipientError {
                reason: "the string address does not contain exactly a single '@' character"
                    .to_string(),
            });
        }
        let client_half = split[0];
        let gateway_half = split[1];

        let split_client: Vec<_> = client_half.split('.').collect();
        if split_client.len() != 2 {
            return Err(RecipientFormattingError::MalformedRecipientError {
                reason: "the string address does not contain exactly a single '.' character"
                    .to_string(),
            });
        }

        let client_identity = match ClientIdentity::from_base58_string(split_client[0]) {
            Ok(client_id) => client_id,
            Err(err) => return Err(RecipientFormattingError::MalformedIdentityError(err)),
        };

        let client_encryption_key = ClientEncryptionKey::from_base58_string(split_client[1])?;

        let gateway = match NodeIdentity::from_base58_string(gateway_half) {
            Ok(gate_id) => gate_id,
            Err(err) => return Err(RecipientFormattingError::MalformedGatewayError(err)),
        };

        Ok(Recipient {
            client_identity,
            client_encryption_key,
            gateway,
        })
    }
}

// ADDRESS . ENCRYPTION @ GATEWAY_ID
impl std::fmt::Display for Recipient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}@{}",
            self.client_identity.to_base58_string(),
            self.client_encryption_key.to_base58_string(),
            self.gateway.to_base58_string()
        )
    }
}

impl std::fmt::Debug for Recipient {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // use the Display implementation
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl FromStr for Recipient {
    type Err = RecipientFormattingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Recipient::try_from_base58_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_conversion_works() {
        let mut rng = rand::thread_rng();

        let client_id_pair = identity::KeyPair::new(&mut rng);
        let client_enc_pair = encryption::KeyPair::new(&mut rng);
        let gateway_id_pair = identity::KeyPair::new(&mut rng);

        let recipient = Recipient::new(
            *client_id_pair.public_key(),
            *client_enc_pair.public_key(),
            *gateway_id_pair.public_key(),
        );

        let str_recipient = recipient.to_string();
        let recovered_recipient = Recipient::try_from_base58_string(str_recipient).unwrap();

        // as long as byte representation of internal keys are identical, it's all fine
        assert_eq!(
            recipient.client_identity.to_bytes(),
            recovered_recipient.client_identity.to_bytes()
        );
        assert_eq!(
            recipient.client_encryption_key.to_bytes(),
            recovered_recipient.client_encryption_key.to_bytes()
        );
        assert_eq!(
            recipient.gateway.to_bytes(),
            recovered_recipient.gateway.to_bytes()
        );
    }

    #[test]
    fn bytes_conversion_works() {
        let mut rng = rand::thread_rng();

        let client_id_pair = identity::KeyPair::new(&mut rng);
        let client_enc_pair = encryption::KeyPair::new(&mut rng);
        let gateway_id_pair = identity::KeyPair::new(&mut rng);

        let recipient = Recipient::new(
            *client_id_pair.public_key(),
            *client_enc_pair.public_key(),
            *gateway_id_pair.public_key(),
        );

        let bytes_recipient = recipient.to_bytes();
        let recovered_recipient = Recipient::try_from_bytes(bytes_recipient).unwrap();

        // as long as byte representation of internal keys are identical, it's all fine
        assert_eq!(
            recipient.client_identity.to_bytes(),
            recovered_recipient.client_identity.to_bytes()
        );
        assert_eq!(
            recipient.client_encryption_key.to_bytes(),
            recovered_recipient.client_encryption_key.to_bytes()
        );
        assert_eq!(
            recipient.gateway.to_bytes(),
            recovered_recipient.gateway.to_bytes()
        );
    }
}
