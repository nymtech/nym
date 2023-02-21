// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity;
use gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
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

    pub fn stream_map(&mut self) -> &mut StreamMap<String, MixnetMessageReceiver> {
        &mut self.stream_map
    }

    pub fn add_recievers(
        &mut self,
        id: identity::PublicKey,
        message_receiver: MixnetMessageReceiver,
        ack_receiver: AcknowledgementReceiver,
    ) {
        let channel_id = id.to_string();
        self.stream_map.insert(channel_id.clone(), message_receiver);
        self.ack_map.insert(channel_id, ack_receiver);
    }

    pub fn remove_recievers(&mut self, id: &str) {
        self.stream_map.remove(id);
        self.ack_map.remove(id);
    }
}
