use std::{array::TryFromSliceError, collections::VecDeque, ops::Range};

use crate::{
    constants::{DEFAULT_HOPS, MAGIC_SLICE, MIN_PACKET_SIZE, MIX_PARAMS_LEN},
    error::OutfoxError,
    format::{MixCreationParameters, MixStageParameters},
};

use rand::{rngs::OsRng, RngCore};
use sphinx_packet::{
    crypto::PrivateKey,
    packet::builder::DEFAULT_PAYLOAD_SIZE,
    route::{Destination, Node},
};

#[derive(Debug)]
pub struct OutfoxPacket {
    mix_params: MixCreationParameters,
    payload: Vec<u8>,
}

pub struct OutfoxProcessedPacket {
    packet: OutfoxPacket,
    next_address: [u8; 32],
}

impl OutfoxProcessedPacket {
    pub fn new(packet: OutfoxPacket, next_address: [u8; 32]) -> Self {
        OutfoxProcessedPacket {
            packet,
            next_address,
        }
    }

    pub fn into_packet(self) -> OutfoxPacket {
        self.packet
    }

    pub fn next_address(&self) -> &[u8; 32] {
        &self.next_address
    }
}

impl TryFrom<&[u8]> for OutfoxPacket {
    type Error = OutfoxError;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        let (header, payload) = v.split_at(MIX_PARAMS_LEN);
        Ok(OutfoxPacket {
            mix_params: MixCreationParameters::try_from(header)?,
            payload: payload.to_vec(),
        })
    }
}

impl OutfoxPacket {
    pub fn recover_plaintext(&self) -> Result<Vec<u8>, OutfoxError> {
        let plaintext = self.payload()[self.payload_range()].to_vec();
        let mut plaintext = VecDeque::from_iter(plaintext);
        while let Some(0) = plaintext.front() {
            plaintext.pop_front();
        }
        let mut plaintext = plaintext.make_contiguous().to_vec();
        let payload = plaintext.split_off(MAGIC_SLICE.len());
        if plaintext != MAGIC_SLICE {
            Err(OutfoxError::InvalidMagicBytes(plaintext))
        } else {
            Ok(payload)
        }
    }

    pub fn len(&self) -> usize {
        self.mix_params().total_packet_length() + MIX_PARAMS_LEN
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, OutfoxError> {
        let mut bytes = vec![];
        bytes.extend(self.mix_params.to_bytes());
        bytes.extend(self.payload.as_slice());
        Ok(bytes)
    }

    pub fn build<M: AsRef<[u8]>>(
        payload: M,
        route: &[Node; 4],
        destination: &Destination,
        packet_size: Option<usize>,
    ) -> Result<OutfoxPacket, OutfoxError> {
        let mut secret_key = [0; 32];
        OsRng.fill_bytes(&mut secret_key);
        let packet_size = packet_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);
        let packet_size = if packet_size < MIN_PACKET_SIZE {
            MIN_PACKET_SIZE
        } else {
            packet_size
        } + MAGIC_SLICE.len();
        let mix_params = MixCreationParameters::new(packet_size as u16);

        let padding = mix_params.total_packet_length() - payload.as_ref().len() - MAGIC_SLICE.len();
        let mut buffer = vec![0; padding];
        buffer.extend_from_slice(MAGIC_SLICE);
        buffer.extend_from_slice(payload.as_ref());

        // Last node in the route is a gateway, it will decrypt last, and get the final destination address
        let (range, stage_params) = mix_params.get_stage_params(0);
        stage_params.encode_mix_layer(
            &mut buffer[range],
            &secret_key,
            route.last().unwrap().pub_key.as_bytes(),
            destination.address.as_bytes_ref(),
        )?;

        let route = route.iter().rev().collect::<Vec<&Node>>();

        // We've reversed the route, and we iterate pairs of node, first node in the pair is the destination, and the second(last) is the processing node
        // Route: [N1, N2, N3, G]
        // Reverse: [G, N3, N2, N1]
        // Pairs: [(G, N3), (N3, N2), (N2, N1)]
        // We iterate over pairs, and encode the mix layer for each pair
        // For the first pair, we encode the mix layer for N3, and the destination is G
        // For the second pair, we encode the mix layer for N2, and the destination is N3
        // For the third pair, we encode the mix layer for N1, and the destination is N2
        // Entry gateway will simply forward the packet to N1 and processing will continue from there
        for (idx, nodes) in route.windows(2).enumerate() {
            let (range, stage_params) = mix_params.get_stage_params(idx + 1);
            // We know that we'll always get 4 nodes, so we can unwrap here
            let processing_node = nodes.last().unwrap();
            let destination_node = nodes.first().unwrap();
            OsRng.fill_bytes(&mut secret_key);
            stage_params.encode_mix_layer(
                &mut buffer[range],
                &secret_key,
                processing_node.pub_key.as_bytes(),
                destination_node.address.as_bytes_ref(),
            )?;
        }

        Ok(OutfoxPacket {
            mix_params,
            payload: buffer,
        })
    }

    pub fn stage_params(&self, layer_number: usize) -> (Range<usize>, MixStageParameters) {
        self.mix_params().get_stage_params(layer_number)
    }

    pub fn mix_params(&self) -> &MixCreationParameters {
        &self.mix_params
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn payload_range(&self) -> Range<usize> {
        self.stage_params(DEFAULT_HOPS - 1).1.payload_range()
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.payload
    }

    pub fn decode_mix_layer(
        &mut self,
        layer: usize,
        mix_secret_key: &[u8; 32],
    ) -> Result<Vec<u8>, OutfoxError> {
        let (range, params) = self.stage_params(layer);
        let routing_data =
            params.decode_mix_layer(&mut self.payload_mut()[range], mix_secret_key)?;
        Ok(routing_data)
    }

    pub fn update_routing_information(&mut self, layer: usize) -> Result<(), TryFromSliceError> {
        let mut routing_info = self
            .mix_params()
            .routing_information_length_by_stage
            .to_vec();
        routing_info.push(0);
        routing_info.swap_remove(layer);
        self.mix_params.routing_information_length_by_stage = routing_info.as_slice().try_into()?;
        Ok(())
    }

    pub fn is_final_hop(&self) -> bool {
        self.mix_params()
            .routing_information_length_by_stage
            .iter()
            .all(|x| x == &0)
    }

    pub fn decode_next_layer(
        &mut self,
        mix_secret_key: &PrivateKey,
    ) -> Result<[u8; 32], OutfoxError> {
        let mix_secret_key = mix_secret_key.to_bytes();
        let routing_lenght_by_stage = self
            .mix_params()
            .routing_information_length_by_stage
            .as_slice();
        let mut layer = DEFAULT_HOPS - 1;
        for (i, length) in routing_lenght_by_stage.iter().rev().enumerate() {
            if length == &32 {
                layer = DEFAULT_HOPS - 1 - i;
                break;
            }
        }
        self.decode_mix_layer(layer, &mix_secret_key)?;
        self.update_routing_information(layer)?;
        let (range, stage_params) = self.mix_params().get_stage_params(layer);
        let routing_bytes = &self.payload()[range][stage_params.routing_data_range()];
        let routing_address: [u8; 32] = routing_bytes.try_into()?;
        Ok(routing_address)
    }
}
