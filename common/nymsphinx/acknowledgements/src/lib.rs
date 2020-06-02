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

pub mod identifier;
pub mod surb_ack;

pub use identifier::AckAes128Key;

// use crate::chunking::fragment::{Fragment, FragmentIdentifier};
// use sphinx::SphinxPacket;
// use std::collections::HashMap;
// use std::time;
//
// pub type ReplySURB = SphinxPacket;
//
// // we need to keep entire payload to resend anyway
// #[derive(Debug)]
// pub struct Acknowledgement {
//     expire: time::Instant,
//     data: Fragment, // or maybe just raw Vec<u8>?
//                     // new idea: change whole thing to a future?
// }
//
// #[derive(Default, Debug)]
// pub struct AcknowledgementReceiver {
//     pending_acks: HashMap<FragmentIdentifier, Acknowledgement>,
// }
//
// impl AcknowledgementReceiver {
//     pub fn new() -> Self {
//         Default::default()
//     }
//
//     pub fn new_acknowledgement(&mut self) {}
//
//     fn resend_fragment(&mut self) {}
// }
