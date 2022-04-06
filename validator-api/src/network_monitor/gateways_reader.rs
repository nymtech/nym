// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity;
use futures::stream::Stream;
use futures::task::Context;
use gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use std::pin::Pin;
use std::task::Poll;
use tokio_stream::StreamMap;

/// Constant used to determine maximum number of times the GatewayReader can poll. It basically
/// tries to solve the same problem that `FuturesUnordered` has: https://github.com/rust-lang/futures-rs/issues/2047
const YIELD_EVERY: usize = 32;

// TODO: Originally I set it to (identity::PublicKey, Vec<Vec<u8>>) and I definitely
// had a reason for doing so, but right now I can't remember what that was...
pub(crate) type GatewayMessages = Vec<Vec<u8>>;

pub(crate) struct GatewayChannel {
    id: identity::PublicKey,
    message_receiver: MixnetMessageReceiver,
    ack_receiver: AcknowledgementReceiver,
    is_closed: bool,
}

impl GatewayChannel {
    pub(crate) fn new(
        id: identity::PublicKey,
        message_receiver: MixnetMessageReceiver,
        ack_receiver: AcknowledgementReceiver,
    ) -> Self {
        GatewayChannel {
            id,
            message_receiver,
            ack_receiver,
            is_closed: false,
        }
    }

    pub fn id(&self) -> &identity::PublicKey {
        &self.id
    }
}

impl Stream for GatewayChannel {
    type Item = Vec<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_closed {
            return Poll::Ready(None);
        }

        let mut polled = 0;

        // empty the ack channel if anything is on it (we don't care about the content at all at the
        // moment)
        while let Poll::Ready(_ack) = Pin::new(&mut self.ack_receiver).poll_next(cx) {
            polled += 1;
        }

        let item = futures::ready!(Pin::new(&mut self.message_receiver).poll_next(cx));

        match item {
            None => Poll::Ready(None),
            // if we managed to get an item, try to also read additional ones
            Some(mut messages) => {
                while let Poll::Ready(new_item) = Pin::new(&mut self.message_receiver).poll_next(cx)
                {
                    polled += 1;
                    match new_item {
                        None => {
                            self.is_closed = true;
                            cx.waker().wake_by_ref();
                            return Poll::Ready(Some(messages));
                        }
                        Some(mut additional_messages) => {
                            messages.append(&mut additional_messages);

                            // it is fine enough to use the same constant here as in the main GatewayReader
                            // as only a single channel, i.e. the main gateway will be capable
                            // of returning more than 2 values
                            if polled >= YIELD_EVERY {
                                cx.waker().wake_by_ref();
                                break;
                            }
                        }
                    }
                }
                Poll::Ready(Some(messages))
            }
        }
    }
}

pub(crate) struct GatewaysReader {
    // latest_read: usize,
    // channels: FuturesUnordered<GatewayChannel>,
    // channels: Vec<GatewayChannel>,
    // waker: Option<Waker>,
    ack_map: StreamMap<String, AcknowledgementReceiver>,
    stream_map: StreamMap<String, MixnetMessageReceiver>,
}

impl GatewaysReader {
    pub(crate) fn new() -> Self {
        GatewaysReader {
            ack_map: StreamMap::new(),
            // waker: None,
            stream_map: StreamMap::new(),
        }
    }

    pub fn stream_map(&mut self) -> &mut StreamMap<String, MixnetMessageReceiver> {
        &mut self.stream_map
    }

    pub fn add_recievers(&mut self, channel: GatewayChannel) {
        let channel_id = channel.id().to_string();
        self.stream_map
            .insert(channel_id.clone(), channel.message_receiver);
        self.ack_map.insert(channel_id, channel.ack_receiver);
    }

    pub fn remove_recievers(&mut self, id: &str) {
        self.stream_map.remove(id);
        self.ack_map.remove(id);
    }
}
