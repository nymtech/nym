// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_fetch::error::MixFetchError;
use futures::channel::oneshot;
use nym_http_requests::socks::MixHttpResponse;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use wasm_utils::{console_error, console_log};

pub type Response = Result<httpcodec::Response<Vec<u8>>, MixFetchError>;
pub type ResponseSender = oneshot::Sender<Response>;
pub type ResponseReceiver = oneshot::Receiver<Response>;

pub type RequestId = u64;

#[derive(Clone, Default)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ResponseSender>>>,
}

impl ActiveRequests {
    pub async fn start_new(&self, id: RequestId, response_sender: ResponseSender) {
        let mut guard = self.inner.lock().await;
        if guard.insert(id, response_sender).is_some() {
            panic!("attempted to insert duplicate request for {id}")
        }
        console_log!("started new request {id}");
    }

    async fn send_response(&self, id: RequestId, response: Response) {
        console_log!("sending response for {id}");
        let mut guard = self.inner.lock().await;

        let Some(sender) = guard.remove(&id) else {
            console_error!("attempted to resolve request {id}, however it no longer exists. Has it been aborted?");
            return;
        };

        // TODO: maybe this one shouldn't panic
        sender
            .send(response)
            .expect("the promise waiting for the response has been dropped")
    }

    pub async fn resolve(&self, response: MixHttpResponse) {
        self.send_response(response.connection_id, Ok(response.http_response))
            .await
    }

    pub async fn reject(&self, id: RequestId, err: MixFetchError) {
        self.send_response(id, Err(err)).await
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
}
