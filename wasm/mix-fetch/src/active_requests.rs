// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixFetchError;
use crate::go_bridge::{goWasmCloseRemoteSocket, goWasmInjectConnError, goWasmInjectServerData};
use crate::RequestId;
use nym_ordered_buffer::OrderedMessageBuffer;
use nym_socks5_requests::SocketData;
use nym_wasm_utils::{console_error, console_log, console_warn};
use rand::{thread_rng, RngCore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ActiveRequest>>>,
}

impl ActiveRequests {
    pub async fn start_new(&self) -> RequestId {
        let mut guard = self.inner.lock().await;
        let req = ActiveRequest::new();
        let mut rng = thread_rng();
        let request_id = loop {
            let candidate = rng.next_u64();
            if !guard.contains_key(&candidate) {
                break candidate;
            }
        };

        console_log!("starting new request {request_id}");

        // it's impossible to insert a duplicate entry here since we're holding the lock for the map
        // and we've generated an id that must have been unique
        guard.insert(request_id, req);

        request_id
    }

    pub async fn invalidate_all(&self) {
        let mut guard = self.inner.lock().await;
        for (id, _req) in guard.drain() {
            let err = MixFetchError::AbortedRequest { request_id: id };
            goWasmInjectConnError(id.to_string(), err.to_string())
        }
    }

    pub async fn get_sending_sequence(&self, id: RequestId) -> Option<u64> {
        let mut guard = self.inner.lock().await;
        if let Some(req) = guard.get_mut(&id) {
            Some(req.next_sending_seq())
        } else {
            console_error!("attempted to get sending sequence of request {id}, however it no longer exists. Has it been aborted?");
            None
        }
    }

    pub async fn finish(&self, id: RequestId) {
        let mut guard = self.inner.lock().await;
        let old = guard.remove(&id);
        if old.is_none() {
            console_error!("attempted to finish request {id}, but it seems to have never existed?")
        }
    }

    pub async fn reject(&self, id: RequestId, err: MixFetchError) {
        let mut guard = self.inner.lock().await;
        let old = guard.remove(&id);
        if old.is_none() {
            console_error!("attempted to reject request {id}, but it no longer exists — likely already cleaned up by Go timeout");
            return;
        }

        goWasmInjectConnError(id.to_string(), err.to_string())
    }

    pub async fn try_send_data_to_go(&self, data: SocketData) {
        let id = data.header.connection_id;
        let mut guard = self.inner.lock().await;
        let Some(req) = guard.get_mut(&id) else {
            // if there's no data and the socket is closed, we're all good because our local
            // must have already closed - this is likely just a retransmitted fragment that
            // arrived after the original
            if !data.data.is_empty() || !data.header.local_socket_closed {
                console_warn!(
                    "received data for request {id} which is no longer active \
                    (likely a retransmitted packet for an already-completed request)"
                );
            }
            return;
        };

        if let Err(err) = req.insert_data(data) {
            console_error!("failed to insert request data: {err}");
            // this unwrap cannot possibly fail as we're holding an exclusive lock for the data
            // and we have just borrowed the content
            guard.remove(&id).unwrap();
            return goWasmCloseRemoteSocket(id.to_string());
        }

        // TODO: clean this one up
        if let Some(contiguous_data) = req.received_data.read() {
            // TODO: deal with closing socket, etc.
            if !contiguous_data.data.is_empty() {
                goWasmInjectServerData(id.to_string(), contiguous_data.data);
            }
            // TODO: that's very crappy way of doing it.
            if Some(contiguous_data.last_sequence) == req.finished_at {
                goWasmCloseRemoteSocket(id.to_string());
            }
        }
    }
}

struct ActiveRequest {
    received_data: OrderedMessageBuffer,
    finished_at: Option<u64>,

    sending_seq: u64,
}

impl ActiveRequest {
    fn new() -> Self {
        ActiveRequest {
            received_data: Default::default(),
            finished_at: None,
            sending_seq: 0,
        }
    }

    fn next_sending_seq(&mut self) -> u64 {
        let next = self.sending_seq;
        self.sending_seq += 1;
        next
    }

    fn insert_data(&mut self, data: SocketData) -> Result<(), MixFetchError> {
        if data.header.local_socket_closed {
            if let Some(already_finished) = self.finished_at {
                return Err(MixFetchError::DuplicateSocketClosure {
                    request: data.header.connection_id,
                    first: already_finished,
                    other: data.header.seq,
                });
            }
            self.finished_at = Some(data.header.seq)
        }

        self.received_data.write(data.header.seq, data.data)?;
        Ok(())
    }
}
