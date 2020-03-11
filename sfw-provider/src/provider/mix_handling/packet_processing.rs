use crate::provider::storage::{ClientStorage, StoreData};
use crypto::encryption;
use log::*;
use mix_client::packet::LOOP_COVER_MESSAGE_PAYLOAD;
use sphinx::payload::Payload;
use sphinx::route::{DestinationAddressBytes, SURBIdentifier};
use sphinx::{ProcessedPacket, SphinxPacket};
use std::io;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    ReceivedForwardHopError,
    NonMatchingRecipient,
    InvalidPayload,
    SphinxProcessingError,
    IOError(String),
}

pub enum MixProcessingResult {
    #[allow(dead_code)]
    ForwardHop,
    FinalHop,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxProcessingError
    }
}

impl From<io::Error> for MixProcessingError {
    fn from(e: io::Error) -> Self {
        use MixProcessingError::*;

        IOError(e.to_string())
    }
}

// PacketProcessor contains all data required to correctly unwrap and store sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    secret_key: Arc<encryption::PrivateKey>,
    client_store: ClientStorage,
}

impl PacketProcessor {
    pub(crate) fn new(secret_key: encryption::PrivateKey, client_store: ClientStorage) -> Self {
        PacketProcessor {
            secret_key: Arc::new(secret_key),
            client_store,
        }
    }

    async fn process_final_hop(
        &self,
        client_address: DestinationAddressBytes,
        surb_id: SURBIdentifier,
        payload: Payload,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        // TODO: should provider try to be recovering plaintext? this would potentially make client retrieve messages of non-constant length,
        // perhaps provider should be re-padding them on retrieval or storing full data?
        let (payload_destination, message) = payload
            .try_recover_destination_and_plaintext()
            .ok_or_else(|| MixProcessingError::InvalidPayload)?;
        if client_address != payload_destination {
            return Err(MixProcessingError::NonMatchingRecipient);
        }

        // we are temporarily ignoring and not storing obvious loop cover traffic messages to
        // not cause our sfw-provider to run out of disk space too quickly.
        // Eventually this is going to get removed and be replaced by a quota system described in:
        // https://github.com/nymtech/nym/issues/137
        if message == LOOP_COVER_MESSAGE_PAYLOAD {
            debug!("Received a loop cover message - not going to store it");
            return Ok(MixProcessingResult::FinalHop);
        }

        let store_data = StoreData::new(client_address, surb_id, message);
        self.client_store.store_processed_data(store_data).await?;

        Ok(MixProcessingResult::FinalHop)
    }

    pub(crate) async fn process_sphinx_packet(
        &self,
        raw_packet_data: [u8; sphinx::PACKET_SIZE],
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let packet = SphinxPacket::from_bytes(&raw_packet_data)?;

        match packet.process(self.secret_key.deref().inner()) {
            Ok(ProcessedPacket::ProcessedPacketForwardHop(_, _, _)) => {
                warn!("Received a forward hop message - those are not implemented for providers");
                Err(MixProcessingError::ReceivedForwardHopError)
            }
            Ok(ProcessedPacket::ProcessedPacketFinalHop(client_address, surb_id, payload)) => {
                self.process_final_hop(client_address, surb_id, payload)
                    .await
            }
            Err(e) => {
                warn!("Failed to unwrap Sphinx packet: {:?}", e);
                Err(MixProcessingError::SphinxProcessingError)
            }
        }
    }
}
