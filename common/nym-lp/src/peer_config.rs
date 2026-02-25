use std::char::MAX;

use rand09::{self, CryptoRng, Rng};

use crate::LpError;

pub const MAX_HOPS: u8 = 16;

pub const LP_PEER_CONFIG_SIZE: usize = 20;

// 20 bytes
pub struct LpPeerConfig {
    // these 3 fields will be packed in one u8
    // with 2 bits left at the end
    hop_id: u8,
    is_exit: bool,
    censorship_resistance: bool,


    // if we add config params later, we can use this
    filler: [u8; 3],

    seed: [u8; 16],
}

impl LpPeerConfig {
   pub fn new<R>(
        rng: &mut R,
        hop_id: u8,
        is_exit: bool,
        censorship_resistance: bool,
    ) -> Result<Self, LpError>
    where
        R: Rng + CryptoRng,
    {
        let seed: [u8; 16] = rng.random();
        let filler: [u8; 3] = rng.random();
        Self::build(hop_id, is_exit, censorship_resistance, seed, filler)
    }

    fn build(
        hop_id: u8,
        is_exit: bool,
        censorship_resistance: bool,
        seed: [u8; 16],
        filler: [u8; 3],
    ) -> Result<Self, LpError> {
        if hop_id >= MAX_HOPS {
            Err(LpError::Internal(format!(
                "Requested hop index ({}) is greater than the allowed maximum {}.",
                hop_id,
                MAX_HOPS - 1
            )))
        } else if is_exit && hop_id == 0 {
            Err(LpError::Internal(
                "Hop index for exit node cannot be zero".into(),
            ))
        } else {
            Ok(Self {
                hop_id,
                is_exit,
                censorship_resistance,

                seed,
                filler,
            })
        }
    }

    pub fn serialize(&self) -> [u8; LP_PEER_CONFIG_SIZE] {
        let mut output_bytes: [u8; LP_PEER_CONFIG_SIZE] = [0u8; LP_PEER_CONFIG_SIZE];
        output_bytes[0..4].copy_from_slice(self.pack_config().as_slice());
        output_bytes[4..].copy_from_slice(&self.seed);
        output_bytes
    }
   pub fn deserialize(bytes: &[u8]) -> Result<Self, LpError> {
        if bytes.len() != LP_PEER_CONFIG_SIZE {
            Err(LpError::DeserializationError(format!(
                "Invalid Lp Config Length ({}), expected ({})",
                bytes.len(),
                LP_PEER_CONFIG_SIZE
            )))
        } else {
            let (hop_id, is_exit, censorship_resistance) = Self::unpack_first_byte(bytes[0]);

            let filler: [u8; 3] = [bytes[1], bytes[2], bytes[3]];
            let mut seed: [u8; 16] = [0u8; 16];
            seed.copy_from_slice(&bytes[4..LP_PEER_CONFIG_SIZE]);
            Self::build(hop_id, is_exit, censorship_resistance, seed, filler)
        }
    }

    fn pack_config(&self) -> [u8; 4] {
        [
            self.pack_first_byte(),
            self.filler[0],
            self.filler[1],
            self.filler[2],
        ]
    }

    fn pack_first_byte(&self) -> u8 {
        let mut byte = self.hop_id;

        if self.is_exit {
            byte |= 0b0001_0000;
        }
        if self.censorship_resistance {
            byte |= 0b0010_0000;
        }

        // there will be 2 free bits at the end

        byte
    }
    fn unpack_first_byte(byte: u8) -> (u8, bool, bool) {
        // extract 4 bits
        let hop_id = byte & 0b0000_1111;

        // extract 5th bit
        let is_exit = (byte & 0b0001_0000) >> 4 == 1;
        // extract 6th bit
        let censorship_resistance = (byte & 0b0010_0000) >> 5 == 1;

        // if we need to use the last 2 bits, we can add something here
        (hop_id, is_exit, censorship_resistance)
    }
}

