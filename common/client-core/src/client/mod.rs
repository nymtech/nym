// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::AtomicU64;

pub mod base_client;
pub mod cover_traffic_stream;
pub(crate) mod helpers;
pub mod inbound_messages;
pub mod key_manager;
pub mod mix_traffic;
pub mod real_messages_control;
pub mod received_buffer;
pub mod replies;
pub mod topology_control;
pub(crate) mod transmission_buffer;

// Packet counters for statistics. These are updated by the various components of the client and
// the packet stats task will periodically read and summarise them.
// Another reason it's useful to have these here is that some components, like the gateway client,
// will check and compare its own packet counters against these to ensure that nothing has been
// lost in the system due e.g async cancellation bugs.
pub(crate) static REAL_PACKETS_SENT: AtomicU64 = AtomicU64::new(0);
pub(crate) static COVER_PACKETS_SENT: AtomicU64 = AtomicU64::new(0);

pub(crate) static REAL_ACKS_RECEIVED: AtomicU64 = AtomicU64::new(0);
pub(crate) static TOTAL_ACKS_RECEIVED: AtomicU64 = AtomicU64::new(0);

pub(crate) static REAL_PACKETS_QUEUED: AtomicU64 = AtomicU64::new(0);
pub(crate) static RETRANSMISSIONS_QUEUED: AtomicU64 = AtomicU64::new(0);
pub(crate) static REPLY_SURB_REQUESTS_QUEUED: AtomicU64 = AtomicU64::new(0);
pub(crate) static ADDITIONAL_REPLY_SURBS_QUEUED: AtomicU64 = AtomicU64::new(0);
