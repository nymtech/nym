use crate::node::metrics;
use addressing::AddressTypeError;
use crypto::encryption;
use log::*;
use sphinx::header::delays::Delay as SphinxDelay;
use sphinx::route::NodeAddressBytes;
use sphinx::{ProcessedPacket, SphinxPacket};
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedFinalHopError,
    SphinxProcessingError,
    InvalidHopAddress,
}

pub enum MixProcessingResult {
    ForwardHop(SocketAddr, Vec<u8>),
    #[allow(dead_code)]
    LoopMessage,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

impl From<addressing::AddressTypeError> for MixProcessingError {
    fn from(_: AddressTypeError) -> Self {
        use MixProcessingError::*;

        InvalidHopAddress
    }
}

// PacketProcessor contains all data required to correctly unwrap and forward sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    secret_key: Arc<encryption::PrivateKey>,
    metrics_reporter: metrics::MetricsReporter,
}

impl PacketProcessor {
    pub(crate) fn new(
        secret_key: encryption::PrivateKey,
        metrics_reporter: metrics::MetricsReporter,
    ) -> Self {
        PacketProcessor {
            secret_key: Arc::new(secret_key),
            metrics_reporter,
        }
    }

    pub(crate) fn report_sent(&self, addr: SocketAddr) {
        self.metrics_reporter.report_sent(addr.to_string())
    }

    async fn process_forward_hop(
        &self,
        packet: SphinxPacket,
        forward_address: NodeAddressBytes,
        delay: SphinxDelay,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let next_hop_address =
            addressing::socket_address_from_encoded_bytes(forward_address.to_bytes())?;

        // Delay packet for as long as required
        tokio::time::delay_for(delay.to_duration()).await;

        Ok(MixProcessingResult::ForwardHop(
            next_hop_address,
            packet.to_bytes(),
        ))
    }

    pub(crate) async fn process_sphinx_packet(
        &self,
        raw_packet_data: [u8; sphinx::PACKET_SIZE],
    ) -> Result<MixProcessingResult, MixProcessingError> {
        // we received something resembling a sphinx packet, report it!
        self.metrics_reporter.report_received();

        let packet = SphinxPacket::from_bytes(&raw_packet_data)?;

        match packet.process(self.secret_key.deref().inner()) {
            Ok(ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay)) => {
                self.process_forward_hop(packet, address, delay).await
            }
            Ok(ProcessedPacket::ProcessedPacketFinalHop(_, _, _)) => {
                warn!("Received a loop cover message that we haven't implemented yet!");
                Err(MixProcessingError::ReceivedFinalHopError)
            }
            Err(e) => {
                warn!("Failed to unwrap Sphinx packet: {:?}", e);
                Err(MixProcessingError::SphinxProcessingError)
            }
        }
    }
}

// TODO: the test that definitely needs to be written is as follows:
// we are stuck trying to write to mix A, can we still forward just fine to mix B?
