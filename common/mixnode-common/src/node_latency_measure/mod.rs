// Copyright 2021 Nym Technologies SA
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
use std::sync::Arc;

pub struct LatencyMeasurer {
    identity: Arc<identity::KeyPair>,
}

impl LatencyMeasurer {
    pub async fn run(&self) {
        //
    }
}

enum PacketType {
    Message,
    ReplyMessage,
}

struct EchoPacket {
    packet_type: PacketType,
    sequence_number: u64,
    // reason for that is so that if we send packet with the same sequence number in the future,
    // nobody is going to be able to replay our old packet because the signature would match
    signature: Vec<u8>,
}

// TODO: reply packet should also be signed by the replier

impl EchoPacket {
    pub fn into_bytes(self) -> Vec<u8> {
        todo!()
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        todo!()
    }

    pub fn construct_reply(self) -> ! {
        todo!()
    }
}
