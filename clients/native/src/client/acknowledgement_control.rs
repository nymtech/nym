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

use crate::client::inbound_messages::InputMessage;
use crate::client::topology_control::TopologyAccessor;
use log::*;
use nymsphinx::acknowledgements::{identifier::recover_identifier, AckAes128Key};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::chunking::{
    fragment::{Fragment, FragmentIdentifier},
    MessageChunker,
};
use nymsphinx::Delay;
use rand::{rngs::OsRng, CryptoRng, Rng};
use std::collections::HashMap;
use std::ops::Deref;
use topology::NymTopology;

struct PendingAcknowledgement {
    fragment: Fragment,
    delay: Delay,
}

struct AcknowledgementController<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    ack_key: AckAes128Key,
    ack_recipient: Recipient,
    pending_acks: HashMap<FragmentIdentifier, PendingAcknowledgement>,
    message_chunker: MessageChunker<R>,
    topology_access: TopologyAccessor<T>,
}

impl<T: NymTopology> AcknowledgementController<OsRng, T> {
    // probably will be received via a channel?
    async fn on_ack(&mut self, ack_content: Vec<u8>) {
        // TODO: ack_key will probably need to be behind an Arc or something

        let frag_id = match recover_identifier(&self.ack_key, &ack_content) {
            None => {
                warn!("Received invalid ACK!"); // should we do anything else about that?
                return;
            }
            Some(frag_id_bytes) => match FragmentIdentifier::try_from_bytes(&frag_id_bytes) {
                Ok(frag_id) => frag_id,
                Err(err) => {
                    warn!("Received invalid ACK! - {:?}", err); // should we do anything else about that?
                    return;
                }
            },
        };

        // TODO: probably cancel underlying future or timer or something... to figure out...

        // but definitely we will at least have to remove the entry from the map
        if self.pending_acks.remove(&frag_id).is_none() {
            warn!("received ACK for packet we haven't stored! - {:?}", frag_id);
        }

        todo!()
    }

    // probably will be received via a channel?
    async fn on_message(&mut self, msg: InputMessage) {
        let (recipient, content) = msg.destruct();
        let split_message = self.message_chunker.split_message(&content);
        let topology_permit = self.topology_access.get_read_permit().await;

        // first we need to deref out of RwLockReadGuard
        // then we need to deref out of TopologyAccessorInner
        // then we must take ref of option, i.e. Option<&T>
        // and finally try to unwrap it to obtain &T
        // then after unwrapping topology, we need to take ref of (borrow) that.
        let topology_ref = (**topology_permit).as_ref().unwrap_or_else(|| todo!());

        // see if it's possible to route the packet to both gateways
        if !topology_ref.can_construct_path_through()
            || !topology_ref.gateway_exists(&recipient.gateway())
            || !topology_ref.gateway_exists(&self.ack_recipient.gateway())
        {
            todo!()
        }

        for message_chunk in split_message {
            // since the paths can be constructed, this CAN'T fail, if it does, there's a bug somewhere
            let id = message_chunk.fragment_identifier();
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let (total_delay, packet) = self
                .message_chunker
                .prepare_chunk_for_sending(chunk_clone, topology_ref, &self.ack_key, &recipient)
                .unwrap();

            // TODO:
            // simulate "sending" of packet to the real traffic stream
            drop(packet);

            // TODO: probably some locking, etc. (but if we were to lock hashmap, we'd do it
            // before the loop to do it only once)
            let tmp_pending_ack = PendingAcknowledgement {
                fragment: message_chunk,
                delay: total_delay,
            };

            self.pending_acks.insert(id, tmp_pending_ack);
        }
    }
}

// required module IO:
// 1. receive from input
// 2. send to real traffic stream
// 3. receive oneshot or notify? from RTS once sent; alternatively maybe mpsc<id> ?
