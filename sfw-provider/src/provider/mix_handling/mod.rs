use crate::provider::storage::StoreData;
use crypto::identity::DummyMixIdentityPrivateKey;
use sphinx::{ProcessedPacket, SphinxPacket};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// TODO: this will probably need to be moved elsewhere I imagine
// DUPLICATE WITH MIXNODE CODE!!!
#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedForwardHopError,
    InvalidPayload,
    NonMatchingRecipient,
    FileIOFailure,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

impl From<std::io::Error> for MixProcessingError {
    fn from(_: std::io::Error) -> Self {
        use MixProcessingError::*;

        FileIOFailure
    }
}

// ProcessingData defines all data required to correctly unwrap sphinx packets
#[derive(Debug, Clone)]
pub(crate) struct MixProcessingData {
    secret_key: DummyMixIdentityPrivateKey,
    pub(crate) store_dir: PathBuf,
}

impl MixProcessingData {
    pub(crate) fn new(secret_key: DummyMixIdentityPrivateKey, store_dir: PathBuf) -> Self {
        MixProcessingData {
            secret_key,
            store_dir,
        }
    }

    pub(crate) fn add_arc_rwlock(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }
}

pub(crate) struct MixPacketProcessor(());

impl MixPacketProcessor {
    pub fn process_sphinx_data_packet(
        packet_data: &[u8],
        processing_data: &RwLock<MixProcessingData>,
    ) -> Result<StoreData, MixProcessingError> {
        let packet = SphinxPacket::from_bytes(packet_data.to_vec())?;
        let read_processing_data = processing_data.read().unwrap();
        let (client_address, client_surb_id, payload) =
            match packet.process(read_processing_data.secret_key.as_scalar()) {
                ProcessedPacket::ProcessedPacketFinalHop(client_address, surb_id, payload) => {
                    (client_address, surb_id, payload)
                }
                _ => return Err(MixProcessingError::ReceivedForwardHopError),
            };

        // TODO: should provider try to be recovering plaintext? this would potentially make client retrieve messages of non-constant length,
        // perhaps provider should be re-padding them on retrieval or storing full data?
        let (payload_destination, message) = payload
            .try_recover_destination_and_plaintext()
            .ok_or_else(|| MixProcessingError::InvalidPayload)?;
        if client_address != payload_destination {
            return Err(MixProcessingError::NonMatchingRecipient);
        }

        Ok(StoreData::new(client_address, client_surb_id, message))
    }
}
