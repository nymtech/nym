// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use futures::channel::oneshot;
use nym_http_requests::socks::MixHttpResponse;
use nym_ordered_buffer::OrderedMessageBuffer;
use nym_socks5_requests::SocketData;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasm_utils::{console_error, console_log};

pub type Response = Result<httpcodec::Response<Vec<u8>>, MixFetchError>;
pub type ResponseSender = oneshot::Sender<Response>;
pub type ResponseReceiver = oneshot::Receiver<Response>;

pub type RequestId = u64;

#[derive(Clone, Default)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ActiveRequest>>>,
}

impl ActiveRequests {
    pub async fn start_new(&self, id: RequestId, response_sender: ResponseSender) {
        let mut guard = self.inner.lock().await;
        let req = ActiveRequest::new(response_sender);
        if guard.insert(id, req).is_some() {
            panic!("attempted to insert duplicate request for {id}")
        }
        console_log!("started new request {id}");
    }

    pub async fn reject(&self, id: RequestId, err: MixFetchError) {
        let mut guard = self.inner.lock().await;
        let Some(req) = guard.remove(&id) else {
            console_error!("attempted to reject request {id}, however it no longer exists. Has it been aborted?");
            return;
        };
        req.reject(err)
    }

    pub async fn abort(&self, id: RequestId) {
        let mut guard = self.inner.lock().await;
        let old = guard.remove(&id);
        if old.is_none() {
            console_error!(
                "attempted to abort request {id}, but it never existed in the first place!"
            )
        }
    }

    pub async fn attempt_resolve(&self, data: SocketData) {
        let id = data.header.connection_id;
        let mut guard = self.inner.lock().await;
        let Some(req) = guard.get_mut(&id) else {
            console_error!("attempted to resolve request {id}, however it no longer exists. Has it been aborted?");
            return;
        };

        if let Err(err) = req.insert_data(data) {
            // this unwrap cannot possibly fail as we're holding an exclusive lock for the data
            // and we have just borrowed the content
            return guard.remove(&id).unwrap().reject(err);
        }

        if req.can_produce_response() {
            // this unwrap cannot possibly fail as we're holding an exclusive lock for the data
            // and we have just borrowed the content
            guard.remove(&id).unwrap().try_resolve()
        }
    }

    pub async fn is_pending(&self, id: RequestId) -> bool {
        let guard = self.inner.lock().await;
        guard.contains_key(&id)
    }
}

struct ActiveRequest {
    response_sender: ResponseSender,
    received_data: OrderedMessageBuffer,
    finished_at: Option<u64>,
}

impl ActiveRequest {
    fn new(response_sender: ResponseSender) -> Self {
        ActiveRequest {
            response_sender,
            received_data: Default::default(),
            finished_at: None,
        }
    }

    fn resolve(self, response: MixHttpResponse) {
        self.response_sender
            .send(Ok(response.http_response))
            .expect("the promise waiting for the response has been dropped")
    }

    fn reject(self, err: MixFetchError) {
        self.response_sender
            .send(Err(err))
            .expect("the promise waiting for the response has been dropped")
    }

    fn try_resolve(mut self) {
        assert!(self.can_produce_response());

        // unwrap here is fine as if `can_produce_response` returns true, the value must be available
        let last_index = self.finished_at.unwrap();
        let Some(reconstructed) = self.received_data.read() else {
            // this implies flaw in our logic as opposed to some malformed data or user error
            // so panic here
            panic!("failed to read reconstructed response data even though it was ready")
        };
        if last_index != reconstructed.last_sequence {
            // same as above, there's nothing user could have done differently
            panic!("mismatched sequence numbers of reconstructed data")
        }

        match MixHttpResponse::try_from_bytes(&reconstructed.data) {
            Ok(mix_http_response) => self.resolve(mix_http_response),
            Err(err) => self.reject(err.into()),
        }
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

    fn can_produce_response(&self) -> bool {
        if let Some(closed) = self.finished_at {
            return self.received_data.can_read_until(closed);
        }

        false
    }
}
