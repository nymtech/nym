// This is still not an ideal home for this struct, because it's not an
// universal nymsphinx addressing method, however, it needs to be
// accessible by both desktop and webassembly client (it's more
// of a helper/utils structure, because before it reaches the gateway
// it's already destructed).

use crate::nodes::{NODE_IDENTITY_SIZE, NodeIdentity};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_sphinx_types::Destination;
use serde::de::{Error as SerdeError, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Formatter};
use std::str::FromStr;
use thiserror::Error;

// Not entirely sure whether this is the correct place for those, but let's see how it's going
// to work out
pub type ClientEncryptionKey = x25519::PublicKey;
const CLIENT_ENCRYPTION_KEY_SIZE: usize = x25519::PUBLIC_KEY_SIZE;

pub type ClientIdentity = ed25519::PublicKey;
const CLIENT_IDENTITY_SIZE: usize = ed25519::PUBLIC_KEY_LENGTH;

pub type RecipientBytes = [u8; Recipient::LEN];

#[derive(Debug, Error)]
pub enum RecipientFormattingError {
    #[error("recipient is malformed - {reason} ")]
    MalformedRecipientError { reason: String },

    #[error("recipient's identity key is malformed: {0}")]
    MalformedIdentityError(ed25519::Ed25519RecoveryError),

    #[error("recipient's encryption key is malformed: {0}")]
    MalformedEncryptionKeyError(#[from] x25519::KeyRecoveryError),

    #[error("recipient gateway's identity key is malformed: {0}")]
    MalformedGatewayError(ed25519::Ed25519RecoveryError),
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
                write!(
                    formatter,
                    "A recipient is 96-byte sequence containing two ed25519 public keys and one x25519 public key all in compressed forms."
                )
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

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // if we know the size hint, check if it matches expectation,
                // otherwise return an error
                if let Some(size_hint) = seq.size_hint()
                    && size_hint != Recipient::LEN
                {
                    return Err(SerdeError::invalid_length(size_hint, &self));
                }

                let mut recipient_bytes = [0u8; Recipient::LEN];

                // clippy's suggestion is completely wrong and it iterates wrong sequence
                #[allow(clippy::needless_range_loop)]
                for i in 0..Recipient::LEN {
                    let Some(elem) = seq.next_element::<u8>()? else {
                        return Err(SerdeError::invalid_length(i + 1, &self));
                    };
                    recipient_bytes[i] = elem;
                }

                // make sure there are no trailing bytes
                if seq.next_element::<u8>()?.is_some() {
                    return Err(SerdeError::invalid_length(Recipient::LEN + 1, &self));
                }

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
    pub fn as_sphinx_destination(&self, trace_id: Option<[u8; 12]>) -> Destination {
        use nym_bin_common::opentelemetry::compact_id_generator::decompress_trace_id;
        let trace_id_16 = if let Some(trace_id) = trace_id {
            decompress_trace_id(&trace_id)
        } else {
            decompress_trace_id(&[0u8; 12])
        };

        // since the nym mix network differs slightly in design from loopix, we do not care
        // about "surb_id" field at all and just use the default value.
        Destination::new(
            self.client_identity.derive_destination_address(),
            trace_id_16,
        )
    }

    pub fn identity(&self) -> &ClientIdentity {
        &self.client_identity
    }

    pub fn encryption_key(&self) -> &ClientEncryptionKey {
        &self.client_encryption_key
    }

    pub fn gateway(&self) -> NodeIdentity {
        self.gateway
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

    fn mock_recipient() -> Recipient {
        Recipient::try_from_bytes([
            67, 5, 132, 146, 3, 236, 116, 89, 254, 57, 131, 159, 69, 181, 55, 208, 12, 108, 136,
            83, 58, 76, 171, 195, 31, 98, 92, 64, 68, 53, 156, 184, 100, 189, 73, 3, 238, 103, 156,
            108, 124, 199, 42, 79, 172, 98, 81, 177, 182, 100, 167, 164, 74, 183, 199, 213, 162,
            173, 102, 112, 30, 159, 148, 66, 44, 75, 230, 182, 138, 114, 170, 163, 209, 82, 204,
            100, 118, 91, 57, 150, 212, 147, 151, 135, 148, 16, 213, 223, 182, 164, 242, 37, 40,
            73, 137, 228,
        ])
        .unwrap()
    }

    #[test]
    fn string_conversion_works() {
        let mut rng = rand::thread_rng();

        let client_id_pair = ed25519::KeyPair::new(&mut rng);
        let client_enc_pair = x25519::KeyPair::new(&mut rng);
        let gateway_id_pair = ed25519::KeyPair::new(&mut rng);

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

        let client_id_pair = ed25519::KeyPair::new(&mut rng);
        let client_enc_pair = x25519::KeyPair::new(&mut rng);
        let gateway_id_pair = ed25519::KeyPair::new(&mut rng);

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

    // calls `visit_bytes`
    #[test]
    fn bincode_serialisation_works() {
        let recipient = mock_recipient();

        #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
        struct MyStruct {
            recipient: Recipient,
        }
        let a = MyStruct { recipient };
        let s = bincode::serialize(&a).unwrap();

        let b = bincode::deserialize(&s).unwrap();

        assert_eq!(a, b);
    }

    // calls `visit_seq`
    #[test]
    fn json_serialisation_works() {
        use serde::{Deserialize, Serialize};

        let recipient = mock_recipient();

        #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
        struct MyStruct {
            recipient: Recipient,
        }
        let a = MyStruct { recipient };
        let s = serde_json::to_string(&a).unwrap();

        let b = serde_json::from_str(&s).unwrap();

        assert_eq!(a, b);
    }
}
