use crate::node::metrics;
use crypto::encryption;
use log::*;
use sphinx::header::delays::Delay as SphinxDelay;
use sphinx::route::NodeAddressBytes;
use sphinx::{ProcessedPacket, SphinxPacket};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    SphinxRecoveryError,
    ReceivedFinalHopError,
    SphinxProcessingError,
    InvalidHopAddress,
}

impl From<sphinx::ProcessingError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of sphinx::ProcessingError
    fn from(_: sphinx::ProcessingError) -> Self {
        use MixProcessingError::*;

        SphinxRecoveryError
    }
}

//
// struct ForwardingData {
//     packet: SphinxPacket,
//     delay: SphinxDelay,
//     recipient: MixPeer,
//     sent_metrics_tx: mpsc::Sender<String>,
// }
//
// // TODO: this will need to be changed if MixPeer will live longer than our Forwarding Data
// impl ForwardingData {
//     fn new(
//         packet: SphinxPacket,
//         delay: SphinxDelay,
//         recipient: MixPeer,
//         sent_metrics_tx: mpsc::Sender<String>,
//     ) -> Self {
//         ForwardingData {
//             packet,
//             delay,
//             recipient,
//             sent_metrics_tx,
//         }
//     }
// }

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

    // async fn wait_and_forward(mut forwarding_data: ForwardingData) {
    //     let delay_duration = Duration::from_nanos(forwarding_data.delay.get_value());
    //     tokio::time::delay_for(delay_duration).await;
    //
    //     if forwarding_data
    //         .sent_metrics_tx
    //         .send(forwarding_data.recipient.stringify())
    //         .await
    //         .is_err()
    //     {
    //         error!("failed to send metrics data to the controller - the underlying thread probably died!");
    //         std::process::exit(1);
    //     }
    //
    //     trace!("RECIPIENT: {:?}", forwarding_data.recipient);
    //     match forwarding_data
    //         .recipient
    //         .send(forwarding_data.packet.to_bytes())
    //         .await
    //     {
    //         Ok(()) => (),
    //         Err(e) => {
    //             warn!(
    //                 "failed to write bytes to next mix peer. err = {:?}",
    //                 e.to_string()
    //             );
    //         }
    //     }
    // }

    fn process_forward_hop(
        &self,
        packet: SphinxPacket,
        forward_address: NodeAddressBytes,
        delay: SphinxDelay,
    ) -> Result<(), MixProcessingError> {
        unimplemented!()
        // let next_mix = match MixPeer::new(next_hop_address) {
        //     Ok(next_mix) => next_mix,
        //     Err(_) => return Err(MixProcessingError::InvalidHopAddress),
        // };
        //
        // let fwd_data = ForwardingData::new(
        //     next_packet,
        //     delay,
        //     next_mix,
        //     processing_data.sent_metrics_tx.clone(),
        // );
        // Ok(fwd_data)

        // TODO: do forwarding here?
    }

    pub(crate) fn process_sphinx_packet(
        &self,
        raw_packet_data: [u8; sphinx::PACKET_SIZE],
    ) -> Result<(), MixProcessingError> {
        // we received something resembling a sphinx packet, report it!
        self.metrics_reporter.report_received();

        let packet = SphinxPacket::from_bytes(&raw_packet_data)?;

        match packet.process(self.secret_key.deref().inner()) {
            Ok(ProcessedPacket::ProcessedPacketForwardHop(packet, address, delay)) => {
                self.process_forward_hop(packet, address, delay)
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
