// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::Stream;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::StreamMap;

pub(crate) enum GatewayMessages {
    Data(Vec<Vec<u8>>),
    Acks(Vec<Vec<u8>>),
}

pub(crate) struct GatewaysReader {
    ack_map: StreamMap<ed25519::PublicKey, AcknowledgementReceiver>,
    stream_map: StreamMap<ed25519::PublicKey, MixnetMessageReceiver>,
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
        id: ed25519::PublicKey,
        message_receiver: MixnetMessageReceiver,
        ack_receiver: AcknowledgementReceiver,
    ) {
        self.stream_map.insert(id, message_receiver);
        self.ack_map.insert(id, ack_receiver);
    }

    pub fn remove_receivers(&mut self, id: ed25519::PublicKey) {
        self.stream_map.remove(&id);
        self.ack_map.remove(&id);
    }
}

impl Stream for GatewaysReader {
    type Item = (ed25519::PublicKey, GatewayMessages);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.ack_map).poll_next(cx) {
            Poll::Ready(None) => {
                // this should have never happened!
                return Poll::Ready(None);
            }
            Poll::Ready(Some(ack_item)) => {
                // wake immediately in case there's an associated data message
                cx.waker().wake_by_ref();
                return Poll::Ready(Some((ack_item.0, GatewayMessages::Acks(ack_item.1))));
            }
            Poll::Pending => (),
        }

        Pin::new(&mut self.stream_map)
            .poll_next(cx)
            .map(|maybe_data_item| {
                maybe_data_item.map(|data_item| (data_item.0, GatewayMessages::Data(data_item.1)))
            })
    }
}
