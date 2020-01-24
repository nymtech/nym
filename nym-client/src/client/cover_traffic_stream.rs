use crate::client::mix_traffic::MixMessage;
use crate::client::topology_control::TopologyInnerRef;
use crate::client::LOOP_COVER_AVERAGE_DELAY;
use futures::channel::mpsc;
use log::{info, trace, warn};
use sphinx::route::Destination;
use std::time::Duration;
use topology::NymTopology;

pub(crate) async fn start_loop_cover_traffic_stream<T: NymTopology>(
    tx: mpsc::UnboundedSender<MixMessage>,
    our_info: Destination,
    topology_ctrl_ref: TopologyInnerRef<T>,
) {
    info!("Starting loop cover traffic stream");
    loop {
        trace!("next cover message!");
        let delay = mix_client::poisson::sample(LOOP_COVER_AVERAGE_DELAY);
        let delay_duration = Duration::from_secs_f64(delay);
        tokio::time::delay_for(delay_duration).await;

        let read_lock = topology_ctrl_ref.read().await;
        let topology = match read_lock.topology.as_ref() {
            None => {
                warn!("No valid topology detected - won't send any loop cover message this time");
                continue;
            }
            Some(topology) => topology,
        };

        let cover_message =
            mix_client::packet::loop_cover_message(our_info.address, our_info.identifier, topology);

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        tx.unbounded_send(MixMessage::new(cover_message.0, cover_message.1))
            .unwrap();
    }
}
