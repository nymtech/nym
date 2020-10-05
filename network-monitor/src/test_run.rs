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

use crate::test_packet::TestPacket;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) type TestRunUpdateSender = UnboundedSender<TestRunUpdate>;
pub(crate) type TestRunUpdateReceiver = UnboundedReceiver<TestRunUpdate>;

pub(crate) struct RunInfo {
    pub nonce: u64,
    pub test_packets: Vec<TestPacket>,
    pub malformed_mixes: Vec<String>,
}

// TODO: need to somehow inform about obviously bad packets too...
pub(crate) enum TestRunUpdate {
    StartSending(RunInfo),
    DoneSending(u64),
}
