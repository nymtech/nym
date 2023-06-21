// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::mix_fetch_client;
use futures::channel::oneshot;
use js_sys::Promise;
use nym_http_requests::socks::MixHttpResponse;
use nym_ordered_buffer::OrderedMessageBuffer;
use nym_socks5_requests::{ConnectionId, SocketData};
use rand::{thread_rng, RngCore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use wasm_utils::{console_error, console_log, console_warn};

pub type Response = Result<httpcodec::Response<Vec<u8>>, MixFetchError>;
pub type ResponseSender = oneshot::Sender<Response>;
pub type ResponseReceiver = oneshot::Receiver<Response>;

pub type RequestId = u64;

#[wasm_bindgen]
pub fn send_client_data(stringified_conn_id: String, data: Vec<u8>) -> Promise {
    console_warn!("got {stringified_conn_id}");
    let conn_id: u64 = stringified_conn_id.parse().expect("TODO: err handling");
    console_warn!("[rust] we got request to send {data:?} from connection {conn_id}");

    future_to_promise(async move {
        // TODO: simplify all of those indirections...
        // this error should be impossible in normal use
        // (unless, of course, user is messing around, but then it's their fault for this panic)
        let mix_fetch = mix_fetch_client().expect("mix fetch hasn't been setup");
        mix_fetch.send_fetch_data(conn_id, data).await?;
        console_log!("rust is done with sending fetch data");
        Ok(JsValue::undefined())
    })
}

#[wasm_bindgen]
pub fn start_go_mix_fetch() -> Promise {
    console_log!("start_go_mix_fetch: start");

    let dummy_id = 42u64.to_string();
    let dummy_endpoint = "https://nymtech.net".to_string();

    future_to_promise(async move {
        console_log!("start_go_mix_fetch: inside future");
        let go_res_fut: JsFuture = goWasmMixFetch(dummy_id, dummy_endpoint).into();
        console_log!("start_go_mix_fetch: about to start dealing with go promise");
        let go_res = go_res_fut.await?;
        console_log!("start_go_mix_fetch: we resolved go promise");
        Ok(go_res)
    })
}

#[wasm_bindgen]
extern "C" {
    pub(crate) fn goWasmMixFetch(raw_connection_id: String, endpoint: String) -> Promise;

    pub(crate) fn goWasmInjectServerData(raw_connection_id: String, data: Vec<u8>);

    pub(crate) fn goWasmCloseRemoteSocket(raw_connection_id: String);
}

#[derive(Clone, Default)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ActiveRequest>>>,
}

impl ActiveRequests {
    pub async fn insert_new(&self, response_sender: ResponseSender) -> RequestId {
        let mut guard = self.inner.lock().await;
        let req = ActiveRequest::new(response_sender);
        let mut rng = thread_rng();
        let request_id = loop {
            let candidate = rng.next_u64();
            if !guard.contains_key(&candidate) {
                break candidate;
            }
        };
        // it's impossible to insert a duplicate entry here since we're holding the lock for the map
        // and we generated an id that must have been unique
        guard.insert(request_id, req);

        request_id
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

    #[deprecated]
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

    pub async fn try_send_data_to_go(&self, data: SocketData) {
        let id = data.header.connection_id;
        let mut guard = self.inner.lock().await;
        let Some(req) = guard.get_mut(&id) else {
            console_error!("attempted to resolve request {id}, however it no longer exists. Has it been aborted?");
            // TODO: if it doesn't exist here, make sure to clear Go's memory too
            return;
        };

        if let Err(err) = req.insert_data(data) {
            // this unwrap cannot possibly fail as we're holding an exclusive lock for the data
            // and we have just borrowed the content
            return guard.remove(&id).unwrap().reject(err);
            // TODO: Go memory clear
        }

        if let Some(contiguous_data) = req.received_data.read() {
            // TODO: deal with closing socket, etc.
            if !contiguous_data.data.is_empty() {
                console_log!("injecting data in Go");
                goWasmInjectServerData(id.to_string(), contiguous_data.data);
                console_log!("injected data in Go");
            }
            // TODO: that's very crappy way of doing it.
            if Some(contiguous_data.last_sequence) == req.finished_at {
                console_log!("telling go that the remote socket is closed");
                goWasmCloseRemoteSocket(id.to_string());
                goWasmInjectServerData(id.to_string(), Vec::new());
            }
        }
    }

    pub async fn is_pending(&self, id: RequestId) -> bool {
        let guard = self.inner.lock().await;
        guard.contains_key(&id)
    }
}

struct ActiveRequest {
    // endpoint: String,
    #[deprecated]
    response_sender: ResponseSender,
    received_data: OrderedMessageBuffer,
    finished_at: Option<u64>,

    sending_seq: u64,
}

impl ActiveRequest {
    fn new(response_sender: ResponseSender) -> Self {
        ActiveRequest {
            response_sender,
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
