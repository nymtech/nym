// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity;
use futures::stream::Stream;
use futures::task::Context;
use gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use std::pin::Pin;
use std::task::{Poll, Waker};
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
    stream_map: StreamMap<String, MixnetMessageReceiver>
}

impl GatewaysReader {
    pub(crate) fn new() -> Self {
        GatewaysReader {
            ack_map: StreamMap::new(),
            // waker: None,
            stream_map: StreamMap::new()
        }
    }

    pub fn stream_map(&mut self) -> &mut StreamMap<String, MixnetMessageReceiver> {
        &mut self.stream_map
    }

    // fn remove_nth(&mut self, i: usize) {
    //     self.channels.remove(i);
    // }

    // todo: if we find that this method is called frequently, perhaps the vector should get
    // replaced with different data structure
    // pub(crate) fn remove_by_key(&mut self, key: identity::PublicKey) {
    //     match self.channels.iter().position(|item| item.id == key) {
    //         Some(i) => {
    //             self.channels.remove(i);
    //         }
    //         // this shouldn't ever get thrown, so perhaps a panic would be more in order?
    //         None => error!(
    //             "tried to remove gateway reader {} but it doesn't exist!",
    //             key.to_base58_string()
    //         ),
    //     }
    // }

    // fn poll_nth(
    //     &mut self,
    //     cx: &mut Context<'_>,
    //     i: usize,
    // ) -> Option<Poll<Option<GatewayMessages>>> {
    //     if let Poll::Ready(item) = Pin::new(&mut self.channels[i]).poll_next(cx) {
    //         self.latest_read = i;
    //         match item {
    //             // Some(messages) => return Some(Poll::Ready(Some((self.channels[i].id, messages)))),
    //             Some(messages) => return Some(Poll::Ready(Some(messages))),
    //             // remove dead channel
    //             None => self.remove_nth(i),
    //         }
    //     }
    //     None
    // }

    // pub(crate) fn insert_channel(&mut self, channel: GatewayChannel) {
    //     self.channels.push(channel);
    //     // if let Some(waker) = self.waker.take() {
    //     //     waker.wake()
    //     // }
        
    // }

    pub fn add_recievers(&mut self, channel: GatewayChannel) {
        let channel_id = channel.id().to_string();
        self.stream_map.insert(channel_id.clone(), channel.message_receiver);
        self.ack_map.insert(channel_id, channel.ack_receiver);
    }

    pub fn remove_recievers(&mut self, id: &str) {
        self.stream_map.remove(id);
        self.ack_map.remove(id);
    }
}

// TODO: not sure if this will scale well, but I don't know what would be a good alternative,
// perhaps try to somehow incorporate FuturesUnordered?
// also, perhaps reading should be done in parallel?
// impl Stream for GatewaysReader {
//     // item represents gateway that returned messages alongside the actual messages
//     type Item = GatewayMessages;

//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         if self.latest_read >= self.channels.len() {
//             self.latest_read = 0;
//         }

//         // don't start reading from beginning each time to at least slightly help with the bias
//         for i in self.latest_read..self.channels.len() {
//             if let Some(item) = self.poll_nth(cx, i) {
//                 return item;
//             }
//         }

//         for i in 0..self.latest_read {
//             if let Some(item) = self.poll_nth(cx, i) {
//                 return item;
//             }
//         }

//         // if we have no channels available, store the waker to be woken when a new one is pushed
//         if self.channels.is_empty() {
//             self.waker = Some(cx.waker().clone())
//         }
//         Poll::Pending
//     }
// }
