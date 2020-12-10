// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::test_packet::TestPacket;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) type TestRunUpdateSender = UnboundedSender<TestRunUpdate>;
pub(crate) type TestRunUpdateReceiver = UnboundedReceiver<TestRunUpdate>;

pub(crate) struct RunInfo {
    pub nonce: u64,
    pub test_packets: Vec<TestPacket>,
    pub malformed_mixes: Vec<String>,
    pub incompatible_mixes: Vec<(String, String)>,
}

pub(crate) enum TestRunUpdate {
    StartSending(RunInfo),
    DoneSending(u64),
}
