use serde::{Deserialize, Serialize};

use crate::utilities::codec::{Codec, DecodingError, EncodingError, EphemeraCodec};
use crate::{
    codec::{Decode, Encode},
    utilities::{
        crypto::Certificate,
        hash::{EphemeraHash, EphemeraHasher},
        hash::{Hash, Hasher},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub(crate) struct EphemeraMessage {
    ///Timestamp of the message
    pub(crate) timestamp: u64,
    ///Application specific logical identifier of the message
    pub(crate) label: String,
    ///Application specific data
    pub(crate) data: Vec<u8>,
    ///Signature of the raw message
    pub(crate) certificate: Certificate,
}

impl EphemeraMessage {
    #[cfg(test)]
    pub(crate) fn new(raw_message: RawEphemeraMessage, certificate: Certificate) -> Self {
        Self {
            timestamp: raw_message.timestamp,
            label: raw_message.label,
            data: raw_message.data,
            certificate,
        }
    }

    pub(crate) fn hash_with_default_hasher(&self) -> anyhow::Result<Hash> {
        let mut hasher = Hasher::default();
        self.hash(&mut hasher)?;
        let hash = hasher.finish().into();
        Ok(hash)
    }
}

impl Encode for EphemeraMessage {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}

impl Decode for EphemeraMessage {
    type Output = Self;

    fn decode(bytes: &[u8]) -> Result<Self::Output, DecodingError> {
        Codec::decode(bytes)
    }
}

impl EphemeraHash for EphemeraMessage {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()> {
        state.update(&self.encode()?);
        Ok(())
    }
}

/// Raw message represents all the data what will be signed.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct RawEphemeraMessage {
    pub(crate) timestamp: u64,
    pub(crate) label: String,
    pub(crate) data: Vec<u8>,
}

impl RawEphemeraMessage {
    #[cfg(test)]
    pub(crate) fn new(label: String, data: Vec<u8>) -> Self {
        use crate::utilities::time::EphemeraTime;
        Self {
            timestamp: EphemeraTime::now(),
            label,
            data,
        }
    }
}

impl From<EphemeraMessage> for RawEphemeraMessage {
    fn from(message: EphemeraMessage) -> Self {
        Self {
            timestamp: message.timestamp,
            label: message.label,
            data: message.data,
        }
    }
}

impl Encode for RawEphemeraMessage {
    fn encode(&self) -> Result<Vec<u8>, EncodingError> {
        Codec::encode(&self)
    }
}
