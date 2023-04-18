use std::ops::Range;

use crate::{
    error::OutfoxError,
    format::{MixCreationParameters, MixStageParameters},
};

use sphinx_packet::{packet::builder::DEFAULT_PAYLOAD_SIZE, route::Node};

pub struct OutfoxPacket {
    mix_params: MixCreationParameters,
    payload: Vec<u8>,
}

pub const DEFAULT_ROUTING_INFO_SIZE: usize = 32;

impl OutfoxPacket {
    pub fn build(
        payload: &[u8],
        route: &[Node; 3],
        user_secret_key: &[u8],
    ) -> Result<OutfoxPacket, OutfoxError> {
        let mut mix_params = MixCreationParameters::new(DEFAULT_PAYLOAD_SIZE);

        for node in route.iter() {
            mix_params.add_outer_layer(node.address.as_bytes_ref().len());
        }

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
