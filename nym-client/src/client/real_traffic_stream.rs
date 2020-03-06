use crate::client::mix_traffic::MixMessage;
use crate::client::topology_control::TopologyAccessor;
use crate::client::InputMessage;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::{error, info, trace, warn};
use sphinx::route::Destination;
use std::pin::Pin;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tokio::time;
use topology::NymTopology;

pub(crate) struct OutQueueControl<T: NymTopology> {
    average_packet_delay: Duration,
    average_message_sending_delay: Duration,
    next_delay: time::Delay,
    mix_tx: mpsc::UnboundedSender<MixMessage>,
    input_rx: mpsc::UnboundedReceiver<InputMessage>,
    our_info: Destination,
    topology_access: TopologyAccessor<T>,
}

pub(crate) enum StreamMessage {
    Cover,
    Real(InputMessage),
}

impl<T: NymTopology> Stream for OutQueueControl<T> {
    type Item = StreamMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // it is not yet time to return a message
        if Pin::new(&mut self.next_delay).poll(cx).is_pending() {
            return Poll::Pending;
        };

        // we know it's time to send a message, so let's prepare delay for the next one
        // Get the `now` by looking at the current `delay` deadline
        let now = self.next_delay.deadline();
        let next_poisson_delay = mix_client::poisson::sample(self.average_message_sending_delay);

        // The next interval value is `next_poisson_delay` after the one that just
        // yielded.
        let next = now + next_poisson_delay;
        self.next_delay.reset(next);

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

impl<T: 'static + NymTopology> OutQueueControl<T> {
    pub(crate) fn new(
        mix_tx: mpsc::UnboundedSender<MixMessage>,
        input_rx: mpsc::UnboundedReceiver<InputMessage>,
        our_info: Destination,
        topology_access: TopologyAccessor<T>,
        average_packet_delay: Duration,
        average_message_sending_delay: Duration,
    ) -> Self {
        OutQueueControl {
            average_packet_delay,
            average_message_sending_delay,
            next_delay: time::delay_for(Default::default()),
            mix_tx,
            input_rx,
            our_info,
            topology_access,
        }
    }

    async fn on_message(&mut self, next_message: StreamMessage) {
        trace!("created new message");
        let route = match self.topology_access.random_route().await {
            None => {
                warn!("No valid topology detected - won't send any real or loop message this time");
                // TODO: this creates a potential problem: we can lose real messages if we were
                // unable to get topology, perhaps we should store them in some buffer?
                return;
            }
            Some(route) => route,
        };

        let next_packet = match next_message {
            StreamMessage::Cover => mix_client::packet::loop_cover_message_route(
                self.our_info.address.clone(),
                self.our_info.identifier,
                route,
                self.average_packet_delay,
            ),
            StreamMessage::Real(real_message) => mix_client::packet::encapsulate_message_route(
                real_message.0,
                real_message.1,
                route,
                self.average_packet_delay,
            ),
        };

        let next_packet = match next_packet {
            Ok(message) => message,
            Err(err) => {
                error!(
                    "Somehow we managed to create an invalid traffic message - {:?}",
                    err
                );
                return;
            }
        };

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        self.mix_tx
            .unbounded_send(MixMessage::new(next_packet.0, next_packet.1))
            .unwrap();
    }

    pub(crate) async fn run_out_queue_control(mut self) {
        // we should set initial delay only when we actually start the stream
        self.next_delay = time::delay_for(mix_client::poisson::sample(
            self.average_message_sending_delay,
        ));

        info!("starting out queue controller");
        while let Some(next_message) = self.next().await {
            self.on_message(next_message).await;
        }
    }

    pub(crate) fn start(self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move { self.run_out_queue_control().await })
    }
}
