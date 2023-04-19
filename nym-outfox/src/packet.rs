use std::{convert::TryFrom, ops::Range};

use crate::{
    error::OutfoxError,
    format::{
        groupelementbytes, tagbytes, MixCreationParameters, MixStageParameters, MIX_PARAMS_LEN,
    },
    randombytes,
};

use sphinx_packet::{packet::builder::DEFAULT_PAYLOAD_SIZE, route::Node};

pub const OUTFOX_PACKET_OVERHEAD: usize =
    MIX_PARAMS_LEN + (groupelementbytes() + tagbytes() + DEFAULT_ROUTING_INFO_SIZE as usize) * 3;

#[derive(Debug)]
pub struct OutfoxPacket {
    mix_params: MixCreationParameters,
    payload: Vec<u8>,
}

pub const DEFAULT_ROUTING_INFO_SIZE: u8 = 32;

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
    pub fn len(&self) -> usize {
        self.mix_params().total_packet_length()
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

    pub fn build(
        payload: &[u8],
        route: &[Node; 3],
        packet_size: Option<usize>,
    ) -> Result<OutfoxPacket, OutfoxError> {
        let secret_key = randombytes(32);
        let packet_size = packet_size.unwrap_or(DEFAULT_PAYLOAD_SIZE);
        let mix_params = MixCreationParameters::new(packet_size as u16);

        let padding = mix_params.total_packet_length() - payload.len();
        let mut buffer = vec![0; padding];
        buffer.extend_from_slice(payload);

        for (idx, node) in route.iter().rev().enumerate() {
            let (range, stage_params) = mix_params.get_stage_params(idx);
            stage_params.encode_mix_layer(&mut buffer[range], &secret_key, node)?;
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
