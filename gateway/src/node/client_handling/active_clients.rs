// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use dashmap::DashMap;
use nymsphinx::DestinationAddressBytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// #[derive(Clone)]
// pub(crate) struct ActiveClientHandle(MixMessageSender);
//
// impl ActiveClientHandle {
//     pub(crate) fn is_valid(&self) -> bool {
//         !self.0.is_closed()
//     }
//
//     pub(crate) fn invalidate(&self) {
//         self.0.close_channel()
//     }
//
//     pub(crate) fn send(&self) {}
// }
//
// struct ActiveClientHandleInner {
//     valid: AtomicBool,
//     sender: MixMessageSender,
// }

#[derive(Clone)]
pub(crate) struct ActiveClientsStore(Arc<DashMap<DestinationAddressBytes, MixMessageSender>>);

impl ActiveClientsStore {
    // pub(crate) fn disconnect(&self, client: DestinationAddressBytes) {
    //     if let Some((_, handle)) = self.0.remove(&client) {
    //         handle.invalidate()
    //     }
    // }

    pub(crate) fn get(&self, client: DestinationAddressBytes) -> Option<MixMessageSender> {
        let entry = self.0.get(&client)?;
        let handle = entry.value();

        // if the entry is stale, remove it from the map
        // if handle.is_valid() {
        if !handle.is_closed() {
            Some(handle.clone())
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.0.remove(&client);
            None
        }
    }

    pub(crate) fn insert(&self, client: DestinationAddressBytes, handle: MixMessageSender) {
        let old = self.0.insert(client, handle);
    }

    pub(crate) fn remove_stales(&self) {
        let mut stales = Vec::new();
        for map_ref in self.0.iter() {
            // if !map_ref.value().is_valid() {
            if map_ref.value().is_closed() {
                stales.push(*map_ref.key())
            }
        }
        for stale in stales {
            self.0.remove(&stale);
        }
    }
}
