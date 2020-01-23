use crate::client::mix_traffic::MixMessage;
use crate::client::topology_control::TopologyInnerRef;
use crate::client::{InputMessage, MESSAGE_SENDING_AVERAGE_DELAY};
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::{info, trace, warn};
use sphinx::route::Destination;
use std::pin::Pin;
use std::time::Duration;
use tokio::time;
use topology::NymTopology;

// have a rather low value for test sake
const AVERAGE_PACKET_DELAY: f64 = 0.1;

pub(crate) struct OutQueueControl<T: NymTopology> {
    delay: time::Delay,
    mix_tx: mpsc::UnboundedSender<MixMessage>,
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
    our_info: Destination,
    topology_ctrl_ref: TopologyInnerRef<T>,
}

pub(crate) enum StreamMessage {
    Cover,
    Real(InputMessage),
}

impl<T: NymTopology> Stream for OutQueueControl<T> {
    type Item = StreamMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if Pin::new(&mut self.delay).poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let now = self.delay.deadline();

        let next_poisson_delay =
            Duration::from_secs_f64(mix_client::poisson::sample(MESSAGE_SENDING_AVERAGE_DELAY));

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let next = now + next_poisson_delay;
        self.delay.reset(next);

        // decide what kind of message to send
        match Pin::new(&mut self.input_rx).poll_next(cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            // if there's an actual message - return it
            Poll::Ready(Some(real_message)) => Poll::Ready(Some(StreamMessage::Real(real_message))),

            // otherwise construct a dummy one
            Poll::Pending => Poll::Ready(Some(StreamMessage::Cover)),
        }
    }
}

impl<T: NymTopology> OutQueueControl<T> {
    pub(crate) fn new(
        mix_tx: mpsc::UnboundedSender<MixMessage>,
        input_rx: mpsc::UnboundedReceiver<InputMessage>,
        our_info: Destination,
        topology: TopologyInnerRef<T>,
    ) -> Self {
        let initial_delay = time::delay_for(Duration::from_secs_f64(MESSAGE_SENDING_AVERAGE_DELAY));
        OutQueueControl {
            delay: initial_delay,
            mix_tx,
            input_rx,
            our_info,
            topology_ctrl_ref: topology,
        }
    }

    pub(crate) async fn run_out_queue_control(mut self) {
        info!("starting out queue controller");
        while let Some(next_message) = self.next().await {
            trace!("created new message");
            let read_lock = self.topology_ctrl_ref.read().await;
            let topology = read_lock.topology.as_ref();

            if topology.is_none() {
                warn!("No valid topology detected - won't send any loop cover or real message this time");
                continue;
            }

            let topology = topology.unwrap();

            let next_packet = match next_message {
                StreamMessage::Cover => mix_client::packet::loop_cover_message(
                    self.our_info.address,
                    self.our_info.identifier,
                    topology,
                ),
                StreamMessage::Real(real_message) => mix_client::packet::encapsulate_message(
                    real_message.0,
                    real_message.1,
                    topology,
                    AVERAGE_PACKET_DELAY,
                ),
            };

            // if this one fails, there's no retrying because it means that either:
            // - we run out of memory
            // - the receiver channel is closed
            // in either case there's no recovery and we can only panic
            self.mix_tx
                .unbounded_send(MixMessage::new(next_packet.0, next_packet.1))
                .unwrap();
        }
    }
}
