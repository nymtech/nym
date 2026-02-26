// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpError;
use rand09::{self, CryptoRng, Rng};

pub const MAX_HOPS: u8 = 16;
pub const LP_PEER_CONFIG_SIZE: usize = 20;

const SEED_LEN: usize = 16;
const CONFIG_LEN: usize = 1;
const FILLER_LEN: usize = LP_PEER_CONFIG_SIZE - SEED_LEN - CONFIG_LEN;

// 20 bytes
#[derive(PartialEq)]
pub struct LpPeerConfig {
    // these FILLER_LEN fields will be packed in one u8
    // with 2 bits left at the end

    // determine the hop id.
    // should be 0 if node_initiator is true
    // should be > 1 if is_exit is true
    hop_id: u8,

    // determine if the recipient should be an exit node
    is_exit: bool,

    // determine if we are establishing a node<>node connection
    // should be false if is_exit is true
    node_initiator: bool,

    // enable censorship resistance countermeasures
    censorship_resistance: bool,

    // if we add config params later, we can use this
    filler: [u8; FILLER_LEN],

    seed: [u8; SEED_LEN],
}

impl LpPeerConfig {
    pub fn new_client_to_entry<R>(rng: &mut R, censorship_resistance: bool) -> Self
    where
        R: Rng + CryptoRng,
    {
        Self::build(
            0,
            false,
            false,
            censorship_resistance,
            rng.random(),
            rng.random(),
        )
    }

    pub fn new_client_to_exit<R>(
        rng: &mut R,
        hop_id: u8,
        censorship_resistance: bool,
    ) -> Result<Self, LpError>
    where
        R: Rng + CryptoRng,
    {
        Self::new(rng, hop_id, true, false, censorship_resistance)
    }

    pub fn new_client_to_intermediate<R>(
        rng: &mut R,
        hop_id: u8,
        censorship_resistance: bool,
    ) -> Result<Self, LpError>
    where
        R: Rng + CryptoRng,
    {
        Self::new(rng, hop_id, false, false, censorship_resistance)
    }

    pub fn new_node_to_node<R>(rng: &mut R) -> Result<Self, LpError>
    where
        R: Rng + CryptoRng,
    {
        // no need for censorship resistance between nodes (for now)
        // hop_id between nodes is 0
        Self::new(rng, 0, false, true, false)
    }

    pub fn new<R>(
        rng: &mut R,
        hop_id: u8,
        is_exit: bool,
        node_initiator: bool,
        censorship_resistance: bool,
    ) -> Result<Self, LpError>
    where
        R: Rng + CryptoRng,
    {
        Self::build_checked(
            hop_id,
            is_exit,
            node_initiator,
            censorship_resistance,
            rng.random(),
            rng.random(),
        )
    }
    fn build(
        hop_id: u8,
        is_exit: bool,
        node_initiator: bool,
        censorship_resistance: bool,
        seed: [u8; SEED_LEN],
        filler: [u8; FILLER_LEN],
    ) -> Self {
        Self {
            hop_id,
            is_exit,
            node_initiator,
            censorship_resistance,
            filler,
            seed,
        }
    }
    fn build_checked(
        hop_id: u8,
        is_exit: bool,
        node_initiator: bool,
        censorship_resistance: bool,
        seed: [u8; SEED_LEN],
        filler: [u8; FILLER_LEN],
    ) -> Result<Self, LpError> {
        if node_initiator && is_exit {
            Err(LpError::Internal(
                "A node cannot establish an exit node for itself.".into(),
            ))
        } else if node_initiator && hop_id != 0 {
            Err(LpError::Internal(
                "Hop id in node to node connections must be zero.".into(),
            ))
        } else if !node_initiator && hop_id >= MAX_HOPS {
            Err(LpError::Internal(format!(
                "Requested hop index ({}) is greater than the allowed maximum {}.",
                hop_id,
                MAX_HOPS - 1
            )))
        } else if !node_initiator && is_exit && hop_id == 0 {
            Err(LpError::Internal(
                "Hop id for exit node cannot be zero.".into(),
            ))
        } else if !node_initiator && !is_exit && hop_id == 15 {
            Err(LpError::Internal(
                "The hop with id 15 must be an exit node.".into(),
            ))
        } else {
            Ok(Self::build(
                hop_id,
                is_exit,
                node_initiator,
                censorship_resistance,
                seed,
                filler,
            ))
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
            let (hop_id, is_exit, node_initiator, censorship_resistance) =
                Self::unpack_first_byte(bytes[0]);

            let mut filler: [u8; FILLER_LEN] = [0u8; FILLER_LEN];
            filler.copy_from_slice(&bytes[CONFIG_LEN..CONFIG_LEN + FILLER_LEN]);

            let mut seed: [u8; SEED_LEN] = [0u8; SEED_LEN];
            seed.copy_from_slice(&bytes[CONFIG_LEN + FILLER_LEN..LP_PEER_CONFIG_SIZE]);

            Self::build_checked(
                hop_id,
                is_exit,
                node_initiator,
                censorship_resistance,
                seed,
                filler,
            )
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

        // Set the 5th bit to determine if the node is an exit node
        if self.is_exit {
            byte |= 0b0001_0000;
        }
        // Set the 6th bit to determine if we're establishing a node to node connection
        if self.node_initiator {
            byte |= 0b0010_0000;
        }
        // Set the 7th bit to determine if we should use censorship resistance measures
        if self.censorship_resistance {
            byte |= 0b0100_0000;
        }

        // There will be 1 free bit at the end

        byte
    }

    fn unpack_first_byte(byte: u8) -> (u8, bool, bool, bool) {
        // extract 4 bits
        let hop_id = byte & 0b0000_1111;

        // extract 5th bit
        let is_exit = (byte & 0b0001_0000) >> 4 == 1;
        // extract 6th bit
        let node_initiator = (byte & 0b0010_0000) >> 5 == 1;
        // extract 7th bit
        let censorship_resistance = (byte & 0b0100_0000) >> 6 == 1;

        // If we need to use the last bit, we can add something here
        (hop_id, is_exit, node_initiator, censorship_resistance)
    }

    pub fn is_client_entry(&self) -> bool {
        self.hop_id == 0 && !self.is_exit && !self.node_initiator
    }

    pub fn is_client_intermediate_node(&self) -> bool {
        self.hop_id > 0 && !self.is_exit && !self.node_initiator
    }

    pub fn is_client_exit(&self) -> bool {
        self.hop_id > 0 && self.is_exit && !self.node_initiator
    }

    pub fn is_node_to_node(&self) -> bool {
        self.hop_id == 0 && !self.is_exit && self.node_initiator
    }
}

#[cfg(test)]
mod test {
    use crate::peer_config::LpPeerConfig;

    #[test]
    fn test_pack_config() {
        let mut rng = rand09::rng();

        // Node to node, no censorship resistance
        {
            let expected_conf = 0b0010_0000;
            let conf = LpPeerConfig::new(&mut rng, 0, false, true, false).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(
                conf_bytes[0],
                LpPeerConfig::new_node_to_node(&mut rng)
                    .unwrap()
                    .serialize()[0]
            );
            assert!(conf.is_node_to_node());
        }

        // Node to node, with censorship resistance
        {
            let expected_conf = 0b0110_0000;
            let conf = LpPeerConfig::new(&mut rng, 0, false, true, true).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert!(conf.is_node_to_node());
        }

        // Client to Entry, no censorship resistance
        {
            let expected_conf = 0b0000_0000;
            let conf = LpPeerConfig::new(&mut rng, 0, false, false, false).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];
            let conf_alt_first_byte =
                LpPeerConfig::new_client_to_entry(&mut rng, false).serialize()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(conf_bytes[0], conf_alt_first_byte);
            assert!(conf.is_client_entry())
        }

        // Client to Entry, with censorship resistance
        {
            let expected_conf = 0b0100_0000;
            let conf = LpPeerConfig::new(&mut rng, 0, false, false, true).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];
            let conf_alt_first_byte =
                LpPeerConfig::new_client_to_entry(&mut rng, true).serialize()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(conf_bytes[0], conf_alt_first_byte);
            assert!(conf.is_client_entry());
        }

        // Client to Exit(exit hop = 1), with censorship resistance
        {
            let expected_conf = 0b0101_0001;
            let conf = LpPeerConfig::new(&mut rng, 1, true, false, true).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];
            let conf_alt_first_byte = LpPeerConfig::new_client_to_exit(&mut rng, 1, true)
                .unwrap()
                .serialize()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(conf_bytes[0], conf_alt_first_byte);
            assert!(conf.is_client_exit());
        }

        // Client to Exit(exit hop = 2), without censorship resistance
        {
            let expected_conf = 0b0001_0010;
            let conf = LpPeerConfig::new(&mut rng, 2, true, false, false).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];
            let conf_alt_first_byte = LpPeerConfig::new_client_to_exit(&mut rng, 2, false)
                .unwrap()
                .serialize()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(conf_bytes[0], conf_alt_first_byte);
            assert!(conf.is_client_exit());
        }
        // Client to Intermediate (hop_id = 14), without censorship resistance
        {
            let expected_conf = 0b0000_1110;
            let conf = LpPeerConfig::new(&mut rng, 14, false, false, false).unwrap();
            let conf_bytes = conf.serialize();
            let deserialized_conf_first_byte = LpPeerConfig::deserialize(&conf_bytes)
                .unwrap()
                .pack_config()[0];
            let conf_alt_first_byte = LpPeerConfig::new_client_to_intermediate(&mut rng, 14, false)
                .unwrap()
                .serialize()[0];

            assert_eq!(expected_conf, conf_bytes[0]);
            assert_eq!(expected_conf, deserialized_conf_first_byte);
            assert_eq!(conf_bytes[0], conf_alt_first_byte);
            assert!(conf.is_client_intermediate_node());
        }
    }
}
