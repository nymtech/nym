// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::identity;
use futures::stream::Stream;
use futures::task::Context;
use gateway_client::MixnetMessageReceiver;
use std::pin::Pin;
use std::task::Poll;

struct GatewayChannel {
    id: identity::PublicKey,
    channel: MixnetMessageReceiver,
    is_closed: bool,
}

impl Stream for GatewayChannel {
    type Item = Vec<Vec<u8>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_closed {
            return Poll::Ready(None);
        }
        // if we managed to get an item, try to also read the next one
        let item = futures::ready!(Pin::new(&mut self.channel).poll_next(cx));
        match item {
            None => Poll::Ready(None),
            Some(mut messages) => {
                while let Poll::Ready(new_item) = Pin::new(&mut self.channel).poll_next(cx) {
                    match new_item {
                        None => {
                            self.is_closed = true;
                            cx.waker().wake_by_ref();
                            return Poll::Ready(Some(messages));
                        }
                        Some(mut additional_messages) => {
                            messages.append(&mut additional_messages);
                        }
                    }
                }
                Poll::Ready(Some(messages))
            }
        }
    }
}

struct GatewaysReader {
    latest_read: usize,
    channels: Vec<GatewayChannel>,
}

impl GatewaysReader {
    fn remove_nth(&mut self, i: usize) {
        self.channels.remove(i);
    }
}

// TODO: not sure if this will scale well, but I don't know what would be a good alternative

impl Stream for GatewaysReader {
    // item represents gateway that returned messages alongside the actual messages
    type Item = (identity::PublicKey, Vec<Vec<u8>>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.latest_read >= self.channels.len() {
            self.latest_read = 0;
        }

        // don't start reading from beginning each time to at least slightly help with the bias
        for i in self.latest_read..self.channels.len() {
            if let Poll::Ready(item) = Pin::new(&mut self.channels[i]).poll_next(cx) {
                self.latest_read = i;
                match item {
                    Some(messages) => return Poll::Ready(Some((self.channels[i].id, messages))),
                    // remove dead channel
                    None => self.remove_nth(i),
                }
            }
        }

        for i in 0..self.latest_read {
            if let Poll::Ready(item) = Pin::new(&mut self.channels[i]).poll_next(cx) {
                self.latest_read = i;
                match item {
                    Some(messages) => return Poll::Ready(Some((self.channels[i].id, messages))),
                    // remove dead channel
                    None => self.remove_nth(i),
                }
            }
        }

        Poll::Pending
    }
}
