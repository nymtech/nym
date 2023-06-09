// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::oneshot;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

pub type Response = Result<httpcodec::Response<Vec<u8>>, RequestError>;
pub type ResponseSender = oneshot::Sender<Response>;
pub type ResponseReceiver = oneshot::Receiver<Response>;

pub type RequestId = u64;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("request {id} timed out after {timeout:?}")]
    Timeout { id: RequestId, timeout: Duration },
}

#[derive(Clone)]
pub struct ActiveRequests {
    // TODO: think whether we need sync or async mutex here
    inner: Arc<Mutex<HashMap<RequestId, ResponseSender>>>,
}

impl ActiveRequests {
    pub async fn new(&self, id: RequestId, response_sender: ResponseSender) {
        let mut guard = self.inner.lock().await;
        if guard.insert(id, response_sender).is_some() {
            panic!("attempted to insert duplicate request for {id}")
        }
    }

    async fn send_response(&self, id: RequestId, response: Response) {
        let mut guard = self.inner.lock().await;
        let sender = guard
            .remove(&id)
            .expect(&*format!("request {id} does not exist!"));

        // TODO: maybe this one shouldn't panic
        sender
            .send(response)
            .expect("the promise waiting for the response has been dropped")
    }

    pub async fn resolve(&self, id: RequestId, data: httpcodec::Response<Vec<u8>>) {
        self.send_response(id, Ok(data)).await
    }

    pub async fn reject(&self, id: RequestId, err: RequestError) {
        self.send_response(id, Err(err)).await
    }
}
