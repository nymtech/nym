use crate::client::mix_traffic::MixMessage;
use crate::client::{InputMessage, MESSAGE_SENDING_AVERAGE_DELAY};
use directory_client::presence::Topology;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::{debug, info, trace};
use sphinx::route::Destination;
use sphinx::SphinxPacket;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::time;

// have a rather low value for test sake
const AVERAGE_PACKET_DELAY: f64 = 0.1;

pub(crate) struct OutQueueControl {
    delay: time::Delay,
    mix_tx: mpsc::UnboundedSender<MixMessage>,
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
    our_info: Destination,

    // due to pinning, DerefMut trait, futures, etc its way easier to
    // just have concrete implementation here rather than generic NymTopology
    // considering that it will be replaced with refreshing topology within few days anyway
    topology: Topology,
}

impl Stream for OutQueueControl {
    type Item = (SocketAddr, SphinxPacket);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if Future::poll(Pin::new(&mut self.delay), cx).is_pending() {
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
        match Stream::poll_next(Pin::new(&mut self.input_rx), cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            // if there's an actual message - return it
            Poll::Ready(Some(real_message)) => {
                trace!("real message");
                Poll::Ready(Some(mix_client::packet::encapsulate_message(
                    real_message.0,
                    real_message.1,
                    &self.topology,
                    AVERAGE_PACKET_DELAY,
                )))
            }

            // otherwise construct a dummy one
            _ => {
                trace!("loop cover message");
                Poll::Ready(Some(mix_client::packet::loop_cover_message(
                    self.our_info.address,
                    self.our_info.identifier,
                    &self.topology,
                )))
            }
        }
    }
}

impl OutQueueControl {
    pub(crate) fn new(
        mix_tx: mpsc::UnboundedSender<MixMessage>,
        input_rx: mpsc::UnboundedReceiver<InputMessage>,
        our_info: Destination,
        topology: Topology,
    ) -> Self {
        let initial_delay = time::delay_for(Duration::from_secs_f64(MESSAGE_SENDING_AVERAGE_DELAY));
        OutQueueControl {
            delay: initial_delay,
            mix_tx,
            input_rx,
            our_info,
            topology,
        }
    }

    pub(crate) async fn run_out_queue_control(mut self) {
        info!("starting out queue controller");
        while let Some(next_message) = self.next().await {
            debug!("created new message");
            // if this one fails, there's no retrying because it means that either:
            // - we run out of memory
            // - the receiver channel is closed
            // in either case there's no recovery and we can only panic
            self.mix_tx
                .unbounded_send(MixMessage::new(next_message.0, next_message.1))
                .unwrap();
        }
    }
}
