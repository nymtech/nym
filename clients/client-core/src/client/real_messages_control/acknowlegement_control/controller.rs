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

use crate::client::real_messages_control::acknowlegement_control::ack_delay_queue::AckDelayQueue;
use crate::client::real_messages_control::acknowlegement_control::RetransmissionRequestSender;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::chunking::fragment::{Fragment, FragmentIdentifier};
use nymsphinx::Delay;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio::time::delay_queue::{self, Expired};
use tokio::time::Error as TimerError;

#[derive(Debug)]
struct PendingAcknowledgement {
    message_chunk: Fragment,
    delay: Delay,
    recipient: Recipient,
    // or perhaps put the delay queue key here?
    // active_reader: Option<Notify>, // retransmission_cancel: Arc<Notify>,
}

// and state values: done, not done, not valid, etc
// AtomicU8

// Condvar?

// Arc::clone, vs Arc::downgrade and then weak.upgrade is difference of 15ns vs 30ns...

// just tokio select with action or delay fired; on delay might as well send READ data!

enum Action {
    /// Inserts new `PendingAcknowledgement`s into the 'shared' state.
    /// Initiated by `InputMessageListener`
    InsertPending(Vec<PendingAcknowledgement>),

    /// Starts the retransmission timer on given `PendingAcknowledgement` with the provided `Duration`.
    /// Initiated by `SentNotificationListener`
    StartTimer(FragmentIdentifier, Duration),

    /// Removes given `PendingAcknowledgement` from the 'shared' state. Also cancels the retransmission timer.
    /// Initiated by `AcknowledgementListener`
    RemovePending(FragmentIdentifier),

    // TODO: this might go away if we send all data on timeout
    /// Reads the `PendingAcknowledgement` data.
    /// Initiated by `RetransmissionRequestListener`
    ReadPending(
        FragmentIdentifier,
        oneshot::Sender<Option<Weak<PendingAcknowledgement>>>, // to send reply on
    ),

    /// Resets the retransmission timer on given `PendingAcknowledgement` with the provided `Duration`.
    /// Returns a bool to indicate whether the action was successful.
    /// Initiated by `RetransmissionRequestListener`
    // return channel is provided so that `RetransmissionRequestListener` could wait until the
    // request was processed and see if it's still valid (i.e. ack wasn't removed between read and update)
    ResetTimer(FragmentIdentifier, Duration, oneshot::Sender<bool>),
}

struct Controller {
    /// Contains a map between `FragmentIdentifier` and its full `PendingAcknowledgement` as well as
    /// key to its `AckDelayQueue` entry if it was started.
    pending_acks_data:
        HashMap<FragmentIdentifier, (Arc<PendingAcknowledgement>, Option<delay_queue::Key>)>,

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

impl Controller {
    fn handle_insert(&mut self, pending_acks: Vec<PendingAcknowledgement>) {
        for pending_ack in pending_acks {
            let frag_id = pending_ack.message_chunk.fragment_identifier();
            if self
                .pending_acks_data
                .insert(frag_id, (Arc::new(pending_ack), None))
                .is_some()
            {
                panic!("Tried to insert duplicate pending ack")
            }
        }
    }

    fn handle_start_timer(&mut self, frag_id: FragmentIdentifier, timeout: Duration) {
        if let Some((_, queue_key)) = self.pending_acks_data.get_mut(&frag_id) {
            if queue_key.is_some() {
                // this branch should be IMPOSSIBLE under ANY condition. It would imply starting
                // timer TWICE for the SAME PendingAcknowledgement
                panic!("Tried to start an already started ack timer!")
            }
            let new_queue_key = self.pending_acks_timers.insert(frag_id, timeout);
            *queue_key = Some(new_queue_key)
        } else {
            // TODO: only reason it's a warning is to see how often it's actually being thrown
            // before merging this should be downgraded to debug/trace
            warn!(
                "[DEBUG] Tried to START TIMER on pending ack that is already gone! - {}",
                frag_id
            );
        }
    }

    fn handle_remove(&mut self, frag_id: FragmentIdentifier) {
        match self.pending_acks_data.remove(&frag_id) {
            None => {
                // TODO: only reason it's a warning is to see how often it's actually being thrown
                // before merging this should be downgraded to debug/trace
                warn!(
                    "[DEBUG] Tried to REMOVE pending ack that is already gone! - {}",
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

                    // TODO: only reason it's a warning is to see how often it's actually being thrown
                    // before merging this should be downgraded to debug/trace
                    warn!(
                        "[DEBUG] Tried to REMOVE pending ack without TIMER active - {}",
                        frag_id
                    );
                }
            }
        }
    }

    // TODO: PERHAPS REMOVE IN FAVOUR OF SENDING ALL DATA ON TIMEOUT
    fn handle_read(
        &mut self,
        frag_id: FragmentIdentifier,
        return_sender: oneshot::Sender<Option<Weak<PendingAcknowledgement>>>,
    ) {
        if let Some((pending_ack_data, _)) = self.pending_acks_data.get(&frag_id) {
            return_sender
                .send(Some(Arc::downgrade(pending_ack_data)))
                .unwrap()
        } else {
            // TODO: only reason it's a warning is to see how often it's actually being thrown
            // before merging this should be downgraded to debug/trace
            warn!(
                "[DEBUG] Tried to READ pending ack that is already gone! - {}",
                frag_id
            );
            return_sender.send(None).unwrap()
        }
    }

    fn handle_reset_timer(
        &mut self,
        frag_id: FragmentIdentifier,
        timeout: Duration,
        return_sender: oneshot::Sender<bool>,
    ) {
        if let Some((_, queue_key)) = self.pending_acks_data.get_mut(&frag_id) {
            if queue_key.is_some() {
                // this branch should be IMPOSSIBLE under ANY condition. It would imply starting
                // timer TWICE for the SAME PendingAcknowledgement
                panic!("Tried to reset an already started ack timer!")
            }
            let new_queue_key = self.pending_acks_timers.insert(frag_id, timeout);
            *queue_key = Some(new_queue_key);
            return_sender.send(true).unwrap();
        } else {
            // TODO: only reason it's a warning is to see how often it's actually being thrown
            // before merging this should be downgraded to debug/trace
            warn!(
                "[DEBUG] Tried to UPDATE TIMER on pending ack that is already gone! - {}",
                frag_id
            );
            // request is no longer valid
            return_sender.send(false).unwrap();
        }
    }

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

        if let Some((_, queue_key)) = self.pending_acks_data.get_mut(&frag_id) {
            if queue_key.is_none() {
                // this branch should be IMPOSSIBLE under ANY condition. It would imply the timeout
                // happened before it even started.
                panic!("Ack expired before it was even scheduled!")
            }
            *queue_key = None;
            // TODO: CHANGE TO                 .send(Some(Arc::downgrade(pending_ack_data)))
            self.retransmission_sender.unbounded_send(frag_id).unwrap()
        } else {
            // this shouldn't cause any issues but shouldn't have happened to begin with!
            error!("An already removed pending ack has expired")
        }
    }

    fn process_action(&mut self, action: Action) {}

    async fn run(&mut self) {
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

/*
Normal:
    InsertPending
    StartTimer
    RemovePending

Retransmission normal:
    InsertPending
    StartTimer
    ReadPending
    UpdateTimer
    ...
    ReadPending
    UpdateTimer
    RemovePending


desync example1:
    InsertPending
    RemovePending
    StartTimer

    no problem - StartTimer becomes a noop if entry does not exist

desync example2:
    InsertPending
    StartTimer
    ReadPending
    ...
    backlog
    ...
    RemovePending -> will invalidate read
    UpdateTimer

    should be no problem - UpdateTimer shouldn't be called as `RetransmissionRequestListener`
    should see the reference count indicates he's the only holder so ack was removed
 */
