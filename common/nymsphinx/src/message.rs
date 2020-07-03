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

use nymsphinx_addressing::clients::Recipient;

/// Prepares the message that is to be sent through the mix network by attaching
/// an optional reply-SURB, padding it to appropriate length, encrypting its content,
/// and chunking into appropriate size [`Fragment`]s.
pub struct MessageSender {}

impl MessageSender {
    fn pad_message(&self, message: Vec<u8>) -> Vec<u8> {
        todo!()
    }

    fn shared_key() {}

    fn attach_reply_surb(&self, message: Vec<u8>) -> Vec<u8> {
        todo!()
    }

    fn split_message(&self, message: Vec<u8>) {
        todo!()
    }

    pub fn prepare_message(&self, message: Vec<u8>, recipient: &Recipient) -> Vec<Vec<u8>> {
        todo!()
    }
}

