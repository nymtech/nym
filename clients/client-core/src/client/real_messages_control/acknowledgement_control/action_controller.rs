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

use super::PendingAcknowledgement;
use crate::client::real_messages_control::acknowledgement_control::ack_delay_queue::AckDelayQueue;
use crate::client::real_messages_control::acknowledgement_control::RetransmissionRequestSender;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use log::*;
use nymsphinx::chunking::fragment::FragmentIdentifier;
use nymsphinx::Delay as SphinxDelay;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio::time::delay_queue::{self, Expired};
use tokio::time::Error as TimerError;

pub(crate) type ActionSender = UnboundedSender<Action>;

// The actual data being sent off as well as potential key to the delay queue
type PendingAckEntry = (Arc<PendingAcknowledgement>, Option<delay_queue::Key>);

// we can either:
// - have a completely new set of packets we just sent and need to create entries for
// - received an ack so we want to remove an entry
// - start a retransmission timer for sending the packet into the network (on either first try or retransmission)
// - update the internal sphinx delay of an expired packet
pub(crate) enum Action {
    /// Inserts new `PendingAcknowledgement`s into the 'shared' state.
    /// Initiated by `InputMessageListener`
    InsertPending(Vec<PendingAcknowledgement>),

    /// Removes given `PendingAcknowledgement` from the 'shared' state. Also cancels the retransmission timer.
    /// Initiated by `AcknowledgementListener`
    RemovePending(FragmentIdentifier),

    /// Starts the retransmission timer on given `PendingAcknowledgement` with the `Duration` based on
    /// its internal data.
    /// Initiated by `SentNotificationListener`
    /// Can also be initiated by `RetransmissionRequestListener` in the rare cases of invalid Topology.
    StartTimer(FragmentIdentifier),

    /// Updates the expected delay of given `PendingAcknowledgement` with the new provided `SphinxDelay`.
    /// Initiated by `RetransmissionRequestListener`
    UpdateDelay(FragmentIdentifier, SphinxDelay),
}

impl Action {
    pub(crate) fn new_insert(pending_acks: Vec<PendingAcknowledgement>) -> Self {
        Action::InsertPending(pending_acks)
    }

    pub(crate) fn new_remove(frag_id: FragmentIdentifier) -> Self {
        Action::RemovePending(frag_id)
    }

    pub(crate) fn new_start_timer(frag_id: FragmentIdentifier) -> Self {
        Action::StartTimer(frag_id)
    }

    pub(crate) fn new_update_delay(frag_id: FragmentIdentifier, delay: SphinxDelay) -> Self {
        Action::UpdateDelay(frag_id, delay)
    }
}

/// Configurable parameters of the `ActionController`
pub(super) struct Config {
    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the additive part `b`
    ack_wait_addition: Duration,

    /// Given ack timeout in the form a * BASE_DELAY + b, it specifies the multiplier `a`
    ack_wait_multiplier: f64,
}

impl Config {
    pub(super) fn new(ack_wait_addition: Duration, ack_wait_multiplier: f64) -> Self {
        Config {
            ack_wait_addition,
            ack_wait_multiplier,
        }
    }
}

pub(super) struct ActionController {
    /// Configurable parameters of the `ActionController`
    config: Config,

    /// Contains a map between `FragmentIdentifier` and its full `PendingAcknowledgement` as well as
    /// key to its `AckDelayQueue` entry if it was started.
    pending_acks_data: HashMap<FragmentIdentifier, PendingAckEntry>,

    // This structure ensures that we will EITHER handle expired timer or a received action and NEVER both
    // at the same time hence getting rid of one possible race condition that we suffered from in the
    // previous version.
    /// DelayQueue with all `PendingAcknowledgement` that are waiting to be either received or
    /// retransmitted if their timer fires up.
    pending_acks_timers: AckDelayQueue<FragmentIdentifier>,

    /// Channel for receiving `Action`s from other modules.
    incoming_actions: UnboundedReceiver<Action>,

    /// Channel for notifying `RetransmissionRequestListener` about expired acknowledgements.
    retransmission_sender: RetransmissionRequestSender,
}

impl ActionController {
    pub(super) fn new(
        config: Config,
        retransmission_sender: RetransmissionRequestSender,
    ) -> (Self, ActionSender) {
        let (sender, receiver) = mpsc::unbounded();
        (
            ActionController {
                config,
                pending_acks_data: HashMap::new(),
                pending_acks_timers: AckDelayQueue::new(),
                incoming_actions: receiver,
                retransmission_sender,
            },
            sender,
        )
    }

    fn handle_insert(&mut self, pending_acks: Vec<PendingAcknowledgement>) {
        for pending_ack in pending_acks {
            let frag_id = pending_ack.message_chunk.fragment_identifier();
            trace!("{} is inserted", frag_id);

            if self
                .pending_acks_data
                .insert(frag_id, (Arc::new(pending_ack), None))
                .is_some()
            {
                panic!("Tried to insert duplicate pending ack")
            }
        }
    }

    fn handle_start_timer(&mut self, frag_id: FragmentIdentifier) {
        trace!("{} is starting its timer", frag_id);

        if let Some((pending_ack_data, queue_key)) = self.pending_acks_data.get_mut(&frag_id) {
            if queue_key.is_some() {
                // this branch should be IMPOSSIBLE under ANY condition. It would imply starting
                // timer TWICE for the SAME PendingAcknowledgement
                panic!("Tried to start an already started ack timer!")
            }
            let timeout = (pending_ack_data.delay.clone() * self.config.ack_wait_multiplier)
                .to_duration()
                + self.config.ack_wait_addition;

            let new_queue_key = self.pending_acks_timers.insert(frag_id, timeout);
            *queue_key = Some(new_queue_key)
        } else {
            debug!(
                "Tried to START TIMER on pending ack that is already gone! - {}",
                frag_id
            );
        }
    }

    fn handle_remove(&mut self, frag_id: FragmentIdentifier) {
        trace!("{} is getting removed", frag_id);

        match self.pending_acks_data.remove(&frag_id) {
            None => {
                debug!(
                    "Tried to REMOVE pending ack that is already gone! - {}",
                    frag_id
                );
            }
            Some((_, queue_key)) => {
                if let Some(queue_key) = queue_key {
                    // there are no possible checks here, we must GUARANTEE that we NEVER try
                    // to remove an entry that doesn't exist (and we MUST GUARANTEE that
                    // we do not have a stale key)
                    self.pending_acks_timers.remove(&queue_key);
                // remove timer
                } else {
                    // I'm not 100% sure if having a `None` key is even possible here
                    // (REMOVE would have to be called before START TIMER),
                    debug!(
                        "Tried to REMOVE pending ack without TIMER active - {}",
                        frag_id
                    );
                }
            }
        }
    }

    // initiated basically as a first step of retransmission. At first data has its delay updated
    // (as new sphinx packet was created with new expected delivery time)
    fn handle_update_delay(&mut self, frag_id: FragmentIdentifier, delay: SphinxDelay) {
        trace!("{} is updating its delay", frag_id);
        // TODO: is it possible to solve this without either locking or temporarily removing the value?
        if let Some((pending_ack_data, queue_key)) = self.pending_acks_data.remove(&frag_id) {
            // this Action is triggered by `RetransmissionRequestListener` which held the other potential
            // reference to this Arc. HOWEVER, before the Action was pushed onto the queue, the reference
            // was dropped hence this unwrap is safe.
            let mut inner_data = Arc::try_unwrap(pending_ack_data).unwrap();
            inner_data.update_delay(delay);

            self.pending_acks_data
                .insert(frag_id, (Arc::new(inner_data), queue_key));
        } else {
            debug!(
                "Tried to UPDATE TIMER on pending ack that is already gone! - {}",
                frag_id
            );
        }
    }

    // note: when the entry expires it's automatically removed from pending_acks_timers
    fn handle_expired_ack_timer(
        &mut self,
        expired_ack: Result<Expired<FragmentIdentifier>, TimerError>,
    ) {
        // I'm honestly not sure how to handle it, because getting it means other things in our
        // system are already misbehaving. If we ever see this panic, then I guess we should worry
        // about it. Perhaps just reschedule it at later point?
        let frag_id = expired_ack
            .expect("Tokio timer returned an error!")
            .into_inner();

        trace!("{} has expired", frag_id);

        if let Some((pending_ack_data, queue_key)) = self.pending_acks_data.get_mut(&frag_id) {
            if queue_key.is_none() {
                // this branch should be IMPOSSIBLE under ANY condition. It would imply the timeout
                // happened before it even started.
                panic!("Ack expired before it was even scheduled!")
            }
            *queue_key = None;
            // downgrading an arc and then upgrading vs cloning is difference of 30ns vs 15ns
            // so it's literally a NO difference while it might prevent us from unnecessarily
            // resending data (in maybe 1 in 1 million cases, but it's something)
            self.retransmission_sender
                .unbounded_send(Arc::downgrade(pending_ack_data))
                .unwrap()
        } else {
            // this shouldn't cause any issues but shouldn't have happened to begin with!
            error!("An already removed pending ack has expired")
        }
    }

    fn process_action(&mut self, action: Action) {
        match action {
            Action::InsertPending(pending_acks) => self.handle_insert(pending_acks),
            Action::RemovePending(frag_id) => self.handle_remove(frag_id),
            Action::StartTimer(frag_id) => self.handle_start_timer(frag_id),
            Action::UpdateDelay(frag_id, delay) => self.handle_update_delay(frag_id, delay),
        }
    }

    pub(super) async fn run(&mut self) {
        loop {
            // at some point there will be a global shutdown signal here as the third option
            tokio::select! {
                // we NEVER expect for ANY sender to get dropped so unwrap here is fine
                action = self.incoming_actions.next() => self.process_action(action.unwrap()),
                // pending ack queue Stream CANNOT return a `None` so unwrap here is fine
                expired_ack = self.pending_acks_timers.next() => self.handle_expired_ack_timer(expired_ack.unwrap())
            }
        }
    }
}
