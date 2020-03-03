use crate::client::mix_traffic::MixMessage;
use crate::client::topology_control::TopologyInnerRef;
use futures::channel::mpsc;
use log::{error, info, trace, warn};
use sphinx::route::Destination;
use std::time;
use topology::NymTopology;

pub(crate) async fn start_loop_cover_traffic_stream<T: NymTopology>(
    tx: mpsc::UnboundedSender<MixMessage>,
    our_info: Destination,
    topology_ctrl_ref: TopologyInnerRef<T>,
    average_cover_message_delay_duration: time::Duration,
    average_packet_delay_duration: time::Duration,
) {
    info!("Starting loop cover traffic stream");
    loop {
        trace!("next cover message!");
        let delay_duration = mix_client::poisson::sample(average_cover_message_delay_duration);
        tokio::time::delay_for(delay_duration).await;

        let read_lock = topology_ctrl_ref.read().await;
        let topology = match read_lock.topology.as_ref() {
            None => {
                warn!("No valid topology detected - won't send any loop cover message this time");
                continue;
            }
            Some(topology) => topology,
        };

        let cover_message = match mix_client::packet::loop_cover_message(
            our_info.address.clone(),
            our_info.identifier,
            topology,
            average_packet_delay_duration,
        ) {
            Ok(message) => message,
            Err(err) => {
                error!(
                    "Somehow we managed to create an invalid cover message - {:?}",
                    err
                );
                continue;
            }
        };

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        tx.unbounded_send(MixMessage::new(cover_message.0, cover_message.1))
            .unwrap();
    }
}
