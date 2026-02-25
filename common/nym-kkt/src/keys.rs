// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::KKTError;
use libcrux_psq::handshake::types::PQEncapsulationKey;
use nym_kkt_ciphersuite::KEM;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub use libcrux_ml_kem::mlkem768::{MlKem768KeyPair, MlKem768PrivateKey, MlKem768PublicKey};
pub use libcrux_psq::classic_mceliece as mceliece;
pub use libcrux_psq::handshake::types::{DHKeyPair, DHPrivateKey, DHPublicKey};

/// Wrapper around keys used for the KEM exchange
/// with cheap clones thanks to Arc wrappers
#[derive(Clone)]
pub struct KEMKeys {
    mc_eliece_pk: Arc<mceliece::PublicKey>,
    mc_eliece_sk: Arc<mceliece::SecretKey>,
    ml_kem768_pk: Arc<MlKem768PublicKey>,
    ml_kem768_sk: Arc<MlKem768PrivateKey>,
}

impl Debug for KEMKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KEMKeys")
            .field("mc_eliece", &"<redacted>")
            .field("ml_kem768", &"<redacted>")
            .finish()
    }
}

impl KEMKeys {
    pub fn new(mc_eliece: mceliece::KeyPair, ml_kem768: MlKem768KeyPair) -> Self {
        let (ml_kem768_sk, ml_kem768_pk) = ml_kem768.into_parts();
        Self {
            mc_eliece_pk: Arc::new(mc_eliece.pk),
            mc_eliece_sk: Arc::new(mc_eliece.sk),
            ml_kem768_pk: Arc::new(ml_kem768_pk),
            ml_kem768_sk: Arc::new(ml_kem768_sk),
        }
    }

    pub fn encoded_encapsulation_key(&self, kem: KEM) -> Option<&[u8]> {
        match kem {
            KEM::McEliece => Some(self.mc_eliece_pk.as_ref().as_ref()),
            KEM::MlKem768 => Some(self.ml_kem768_pk.as_slice()),
            // _ => None,
        }
    }

    pub fn encapsulation_key(&self, kem: KEM) -> Option<EncapsulationKey> {
        match kem {
            KEM::McEliece => Some(EncapsulationKey::McEliece(self.mc_eliece_pk.clone())),
            KEM::MlKem768 => Some(EncapsulationKey::MlKem768(self.ml_kem768_pk.clone())),
            // _ => None,
        }
    }

    pub fn mc_eliece_encapsulation_key(&self) -> &mceliece::PublicKey {
        &self.mc_eliece_pk
    }

    pub fn ml_kem768_encapsulation_key(&self) -> &MlKem768PublicKey {
        self.ml_kem768_pk.as_ref()
    }

    pub fn mc_eliece_decapsulation_key(&self) -> &mceliece::SecretKey {
        &self.mc_eliece_sk
    }

    pub fn ml_kem768_decapsulation_key(&self) -> &MlKem768PrivateKey {
        &self.ml_kem768_sk
    }
}

#[derive(Clone)]
pub enum EncapsulationKey {
    McEliece(Arc<mceliece::PublicKey>),
    MlKem768(Arc<MlKem768PublicKey>),
}

impl Debug for EncapsulationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EncapsulationKey::McEliece(_) => write!(f, "EncapsulationKey::McEliece"),
            EncapsulationKey::MlKem768(_) => write!(f, "EncapsulationKey::MlKem768"),
        }
    }
}

impl EncapsulationKey {
    pub fn kem(&self) -> KEM {
        match self {
            EncapsulationKey::McEliece(_) => KEM::McEliece,
            EncapsulationKey::MlKem768(_) => KEM::MlKem768,
        }
    }

    pub fn as_pq_encapsulation_key(&self) -> PQEncapsulationKey<'_> {
        match self {
            EncapsulationKey::McEliece(pk) => PQEncapsulationKey::CMC(pk),
            EncapsulationKey::MlKem768(pk) => PQEncapsulationKey::MlKem(pk),
        }
    }

    pub fn try_from_bytes(bytes: Vec<u8>, kem: KEM) -> Result<EncapsulationKey, KKTError> {
        match kem {
            KEM::MlKem768 => Ok(EncapsulationKey::MlKem768(Arc::new(
                MlKem768PublicKey::try_from(bytes.as_slice()).map_err(|_| KKTError::KEMError {
                    info: "mlkem768 key of invalid length",
                })?,
            ))),
            KEM::McEliece => {
                let boxed_array: Box<[u8; nym_kkt_ciphersuite::mceliece::PUBLIC_KEY_LENGTH]> =
                    bytes
                        .into_boxed_slice()
                        .try_into()
                        .map_err(|_| KKTError::KEMError {
                            info: "mceliece key of invalid length",
                        })?;

                Ok(EncapsulationKey::McEliece(Arc::new(
                    mceliece::PublicKey::from(boxed_array),
                )))
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            EncapsulationKey::McEliece(k) => k.as_ref().as_ref(),
            EncapsulationKey::MlKem768(k) => k.as_ref().as_ref(),
        }
    }
}

// storage helpers
pub mod storage_wrappers {
    use nym_pemstore::traits::PemStorableKey;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum MalformedStoredKeyError {
        #[error("{typ} stored key has an invalid length")]
        InvalidKeyLength { typ: &'static str },

        #[error("{typ} stored key is malformed: {message}")]
        MalformedData { typ: &'static str, message: String },

        #[error("attempted to take ownership of a stored {typ} key representation")]
        IllegalStoredConversion { typ: &'static str },
    }

    pub trait StorableKey: Sized {
        type StorableRepresentation<'a>: PemStorableKey
            + From<&'a Self>
            + TryInto<Self, Error = MalformedStoredKeyError>
            + Sized
        where
            Self: 'a;

        fn to_storable(&self) -> Self::StorableRepresentation<'_> {
            self.into()
        }

        fn from_storable(
            repr: Self::StorableRepresentation<'_>,
        ) -> Result<Self, MalformedStoredKeyError> {
            repr.try_into()
        }
    }

    macro_rules! declare_key_wrappers {
        ($pub_key_type:ty, $private_key_type:ty) => {
            pub enum StorablePublicKey<'a> {
                Owned(Box<$pub_key_type>),
                Borrowed(&'a $pub_key_type),
            }

            impl AsRef<$pub_key_type> for StorablePublicKey<'_> {
                fn as_ref(&self) -> &$pub_key_type {
                    match self {
                        StorablePublicKey::Owned(k) => k,
                        StorablePublicKey::Borrowed(k) => k,
                    }
                }
            }

            pub enum StorablePrivateKey<'a> {
                Owned(Box<$private_key_type>),
                Borrowed(&'a $private_key_type),
            }

            impl AsRef<$private_key_type> for StorablePrivateKey<'_> {
                fn as_ref(&self) -> &$private_key_type {
                    match self {
                        StorablePrivateKey::Owned(k) => k,
                        StorablePrivateKey::Borrowed(k) => k,
                    }
                }
            }

            impl<'a> From<&'a $pub_key_type> for StorablePublicKey<'a> {
                fn from(value: &'a $pub_key_type) -> Self {
                    StorablePublicKey::Borrowed(value)
                }
            }

            impl<'a> TryFrom<StorablePublicKey<'a>> for $pub_key_type {
                type Error = MalformedStoredKeyError;

                fn try_from(value: StorablePublicKey<'a>) -> Result<Self, Self::Error> {
                    match value {
                        StorablePublicKey::Owned(value) => Ok(*value),
                        StorablePublicKey::Borrowed(_) => {
                            Err(MalformedStoredKeyError::IllegalStoredConversion {
                                typ: <StorablePublicKey as PemStorableKey>::pem_type(),
                            })
                        }
                    }
                }
            }

            impl<'a> From<&'a $private_key_type> for StorablePrivateKey<'a> {
                fn from(value: &'a $private_key_type) -> Self {
                    StorablePrivateKey::Borrowed(value)
                }
            }

            impl<'a> TryFrom<StorablePrivateKey<'a>> for $private_key_type {
                type Error = MalformedStoredKeyError;

                fn try_from(value: StorablePrivateKey<'a>) -> Result<Self, Self::Error> {
                    match value {
                        StorablePrivateKey::Owned(value) => Ok(*value),
                        StorablePrivateKey::Borrowed(_) => {
                            Err(MalformedStoredKeyError::IllegalStoredConversion {
                                typ: <StorablePrivateKey as PemStorableKey>::pem_type(),
                            })
                        }
                    }
                }
            }

            impl $crate::keys::storage_wrappers::StorableKey for $pub_key_type {
                type StorableRepresentation<'a> = StorablePublicKey<'a>;
            }

            impl $crate::keys::storage_wrappers::StorableKey for $private_key_type {
                type StorableRepresentation<'a> = StorablePrivateKey<'a>;
            }
        };
    }

    pub mod mceliece {
        use crate::keys::storage_wrappers::MalformedStoredKeyError;
        use libcrux_psq::classic_mceliece;
        use nym_pemstore::traits::PemStorableKey;

        declare_key_wrappers!(classic_mceliece::PublicKey, classic_mceliece::SecretKey);

        impl<'a> PemStorableKey for StorablePrivateKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "MCELIECE PRIVATE KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_ref().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let bytes: Box<[u8; nym_kkt_ciphersuite::mceliece::SECRET_KEY_LENGTH]> =
                    bytes.to_vec().into_boxed_slice().try_into().map_err(|_| {
                        MalformedStoredKeyError::InvalidKeyLength {
                            typ: Self::pem_type(),
                        }
                    })?;

                Ok(StorablePrivateKey::Owned(Box::new(
                    classic_mceliece::SecretKey::from(bytes),
                )))
            }
        }

        impl<'a> PemStorableKey for StorablePublicKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "MCELIECE PUBLIC KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_ref().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let bytes: Box<[u8; nym_kkt_ciphersuite::mceliece::PUBLIC_KEY_LENGTH]> =
                    bytes.to_vec().into_boxed_slice().try_into().map_err(|_| {
                        MalformedStoredKeyError::InvalidKeyLength {
                            typ: Self::pem_type(),
                        }
                    })?;

                Ok(StorablePublicKey::Owned(Box::new(
                    classic_mceliece::PublicKey::from(bytes),
                )))
            }
        }
    }

    pub mod mlkem768 {
        use crate::keys::storage_wrappers::MalformedStoredKeyError;
        use libcrux_ml_kem::mlkem768::{MlKem768PrivateKey, MlKem768PublicKey};
        use nym_pemstore::traits::PemStorableKey;

        declare_key_wrappers!(MlKem768PublicKey, MlKem768PrivateKey);

        impl<'a> PemStorableKey for StorablePrivateKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "MLKEM768 PRIVATE KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_slice().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let inner = MlKem768PrivateKey::try_from(bytes).map_err(|message| {
                    MalformedStoredKeyError::MalformedData {
                        typ: Self::pem_type(),
                        message: message.to_string(),
                    }
                })?;
                Ok(StorablePrivateKey::Owned(Box::new(inner)))
            }
        }

        impl<'a> PemStorableKey for StorablePublicKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "MLKEM768 PUBLIC KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_slice().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let inner = MlKem768PublicKey::try_from(bytes).map_err(|message| {
                    MalformedStoredKeyError::MalformedData {
                        typ: Self::pem_type(),
                        message: message.to_string(),
                    }
                })?;
                Ok(StorablePublicKey::Owned(Box::new(inner)))
            }
        }
    }

    pub mod x25519 {
        use crate::keys::storage_wrappers::MalformedStoredKeyError;
        use libcrux_psq::handshake::types::{DHPrivateKey, DHPublicKey};
        use nym_pemstore::traits::PemStorableKey;

        declare_key_wrappers!(DHPublicKey, DHPrivateKey);

        impl<'a> PemStorableKey for StorablePrivateKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "LP X25519 PRIVATE KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_ref().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let bytes =
                    bytes
                        .try_into()
                        .map_err(|_| MalformedStoredKeyError::InvalidKeyLength {
                            typ: Self::pem_type(),
                        })?;
                Ok(StorablePrivateKey::Owned(Box::new(
                    DHPrivateKey::from_bytes(&bytes).map_err(|err| {
                        MalformedStoredKeyError::MalformedData {
                            typ: Self::pem_type(),
                            message: format!("{err:?}"),
                        }
                    })?,
                )))
            }
        }

        impl<'a> PemStorableKey for StorablePublicKey<'a> {
            type Error = MalformedStoredKeyError;

            fn pem_type() -> &'static str {
                "LP X25519 PUBLIC KEY"
            }

            fn to_bytes(&self) -> Vec<u8> {
                self.as_ref().as_ref().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
                let bytes =
                    bytes
                        .try_into()
                        .map_err(|_| MalformedStoredKeyError::InvalidKeyLength {
                            typ: Self::pem_type(),
                        })?;
                Ok(StorablePublicKey::Owned(Box::new(DHPublicKey::from_bytes(
                    &bytes,
                ))))
            }
        }
    }
}
