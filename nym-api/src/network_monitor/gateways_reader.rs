// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use nym_crypto::asymmetric::identity;
use nym_gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::StreamMap;

pub(crate) type GatewayMessages = Vec<Vec<u8>>;

pub(crate) struct GatewaysReader {
    ack_map: StreamMap<String, AcknowledgementReceiver>,
    stream_map: StreamMap<String, MixnetMessageReceiver>,
}

impl GatewaysReader {
    pub(crate) fn new() -> Self {
        GatewaysReader {
            ack_map: StreamMap::new(),
            stream_map: StreamMap::new(),
        }
    }

    pub fn add_receivers(
        &mut self,
        id: identity::PublicKey,
        message_receiver: MixnetMessageReceiver,
        ack_receiver: AcknowledgementReceiver,
    ) {
        let channel_id = id.to_string();
        self.stream_map.insert(channel_id.clone(), message_receiver);
        self.ack_map.insert(channel_id, ack_receiver);
    }

    pub fn remove_receivers(&mut self, id: &str) {
        self.stream_map.remove(id);
        self.ack_map.remove(id);
    }
}

impl Stream for GatewaysReader {
    // just return whatever is returned by our main `stream_map`
    type Item = <StreamMap<String, MixnetMessageReceiver> as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // exhaust the ack map if possible
        match Pin::new(&mut self.ack_map).poll_next(cx) {
            Poll::Ready(None) => {
                // this should have never happened!
                return Poll::Ready(None);
            }
            Poll::Ready(Some(_item)) => (),
            Poll::Pending => (),
        }

        Pin::new(&mut self.stream_map).poll_next(cx)
    }
}
