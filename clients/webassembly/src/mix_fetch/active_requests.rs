// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use crate::mix_fetch::go_bridge::{goWasmCloseRemoteSocket, goWasmInjectServerData};
use crate::mix_fetch::RequestId;
use futures::channel::oneshot;
use nym_ordered_buffer::OrderedMessageBuffer;
use nym_socks5_requests::SocketData;
use rand::{thread_rng, RngCore};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use wasm_bindgen::JsValue;
use wasm_utils::{console_error, console_log};

type RequestErrorSender = oneshot::Sender<MixFetchError>;

#[derive(Clone, Default)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ActiveRequest>>>,
}

impl ActiveRequests {
    pub async fn start_new(&self, request_error_sender: RequestErrorSender) -> RequestId {
        todo!()
        // let mut guard = self.inner.lock().await;
        // let req = ActiveRequest::new(request_error_sender);
        // let mut rng = thread_rng();
        // let request_id = loop {
        //     let candidate = rng.next_u64();
        //     if !guard.contains_key(&candidate) {
        //         break candidate;
        //     }
        // };
        // // it's impossible to insert a duplicate entry here since we're holding the lock for the map
        // // and we've generated an id that must have been unique
        // guard.insert(request_id, req);
        //
        // request_id
    }

    pub async fn start_new2(&self) -> RequestId {
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

    pub async fn get_sending_sequence(&self, id: RequestId) -> Option<u64> {
        let mut guard = self.inner.lock().await;
        if let Some(req) = guard.get_mut(&id) {
            Some(req.next_sending_seq())
        } else {
            console_error!("attempted to get sending sequence of request {id}, however it no longer exists. Has it been aborted?");
            None
        }
    }

    pub async fn reject(&self, id: RequestId, err: MixFetchError) {
        todo!()
        // let mut guard = self.inner.lock().await;
        // let Some(req) = guard.remove(&id) else {
        //     console_error!("attempted to reject request {id}, however it no longer exists. Has it been aborted?");
        //     return;
        // };
        // req.reject(err);
        // todo!("clear Go memory here")
    }

    pub async fn abort(&self, id: RequestId) {
        let mut guard = self.inner.lock().await;
        let old = guard.remove(&id);
        if old.is_none() {
            console_error!(
                "attempted to abort request {id}, but it never existed in the first place!"
            )
        }
        todo!("clear Go memory here")
    }

    pub async fn finish(&self, id: RequestId) {
        let mut guard = self.inner.lock().await;
        let old = guard.remove(&id);
        if old.is_none() {
            console_error!("attempted to finish request {id}, but it seems to have never existed?")
        }
    }

    pub async fn try_send_data_to_go(&self, data: SocketData) {
        console_log!(
            "sending {} bytes to {}",
            data.header.connection_id,
            data.data.len()
        );

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

            todo!()
            // return guard.remove(&id).unwrap().reject(err);
            // todo!("clear Go memory here")
        }

        // TODO: clean this one up
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
}

struct ActiveRequest {
    // endpoint: String,
    // request_error_sender: RequestErrorSender,
    received_data: OrderedMessageBuffer,
    finished_at: Option<u64>,

    sending_seq: u64,
}

impl ActiveRequest {
    // fn new(request_error_sender: RequestErrorSender) -> Self {
    fn new() -> Self {
        ActiveRequest {
            // request_error_sender,
            received_data: Default::default(),
            finished_at: None,
            sending_seq: 0,
        }
    }

    // fn reject(self, err: MixFetchError) {
    //     if self.request_error_sender.send(err).is_err() {
    //         console_error!("failed to reject the request")
    //     }
    // }

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

struct MixFetchRequest {
    //
}

impl Future for MixFetchRequest {
    type Output = Result<web_sys::Request, MixFetchError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
    }
}
