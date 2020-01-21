use crate::client::mix_traffic::MixMessage;
use crate::client::{InputMessage, MESSAGE_SENDING_AVERAGE_DELAY};
use crate::utils;
use futures::channel::mpsc;
use futures::{select, StreamExt};
use log::{debug, error, info, trace};
use sphinx::route::Destination;
use std::time::Duration;
use topology::NymTopology;

pub(crate) async fn control_out_queue<T>(
    mix_tx: mpsc::UnboundedSender<MixMessage>,
    mut input_rx: mpsc::UnboundedReceiver<InputMessage>,
    our_info: Destination,
    topology: T,
) where
    T: NymTopology,
{
    info!("Starting out queue controller where real traffic (or loop cover if nothing is available) will be sent");
    loop {
        // TODO: consider replacing select macro with our own proper future definition with polling
        let traffic_message = select! {
            real_message = input_rx.next() => {
                debug!("we got a real message!");
                if real_message.is_none() {
                    error!("Unexpected 'None' real message!");
                    std::process::exit(1);
                }
                let real_message = real_message.unwrap();
                trace!("real message: {:?}", real_message);
                utils::sphinx::encapsulate_message(real_message.0, real_message.1, &topology)
            },

            default => {
                debug!("no real message - going to send extra loop cover");
                utils::sphinx::loop_cover_message(our_info.address, our_info.identifier, &topology)
            }
        };

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        mix_tx
            .unbounded_send(MixMessage::new(traffic_message.0, traffic_message.1))
            .unwrap();

        let delay_duration = Duration::from_secs_f64(MESSAGE_SENDING_AVERAGE_DELAY);
        tokio::time::delay_for(delay_duration).await;
    }
}
