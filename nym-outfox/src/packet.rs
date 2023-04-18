use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::{
    error::OutfoxError,
    format::{MixCreationParameters, MixStageParameters, GROUPELEMENTBYTES, TAGBYTES},
};

use sphinx_packet::{packet::builder::DEFAULT_PAYLOAD_SIZE, route::Node};

pub const OUTFOX_PACKET_OVERHEAD: usize =
    3 * DEFAULT_ROUTING_INFO_SIZE + GROUPELEMENTBYTES + TAGBYTES;

#[derive(Serialize, Deserialize)]
pub struct OutfoxPacket {
    mix_params: MixCreationParameters,
    payload: Vec<u8>,
}

pub const DEFAULT_ROUTING_INFO_SIZE: usize = 32;

impl OutfoxPacket {
    pub fn len(&self) -> usize {
        self.mix_params().total_packet_length() + self.payload.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // TODO: Replace with lightweight methods
    pub fn to_bytes(&self) -> Result<Vec<u8>, OutfoxError> {
        bincode::serialize(self).map_err(|_e| OutfoxError::Bincode)
    }

    pub fn from_bytes(v: &[u8]) -> Result<Self, OutfoxError> {
        bincode::deserialize(v).map_err(|_| OutfoxError::Bincode)
    }

    pub fn build(
        payload: &[u8],
        route: &[Node; 3],
        user_secret_key: &[u8],
    ) -> Result<OutfoxPacket, OutfoxError> {
        let mix_params = MixCreationParameters::new(DEFAULT_PAYLOAD_SIZE);

        // for node in route.iter() {
        //     mix_params.add_outer_layer(node.address.as_bytes_ref().len());
        // }

        let padding = mix_params.total_packet_length() - payload.len();
        let mut buffer = vec![0; padding];
        buffer.extend_from_slice(payload);

        for (idx, node) in route.iter().rev().enumerate() {
            let (range, stage_params) = mix_params.get_stage_params(idx);
            stage_params.encode_mix_layer(&mut buffer[range], user_secret_key, node)?;
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
        self.stage_params(2).1.payload_range()
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.payload
    }

    pub fn decode_mix_layer(
        &mut self,
        layer: usize,
        mix_secret_key: &[u8; 32],
    ) -> Result<(), OutfoxError> {
        let (range, params) = self.stage_params(layer);
        params.decode_mix_layer(&mut self.payload_mut()[range], mix_secret_key)?;
        Ok(())
    }
}
