// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on NodeTestResult
#![allow(clippy::drop_non_drop)]

use crate::error::NodeTesterError;
use crate::tester::LockedGatewayClient;
use crate::types::WasmTestMessageExt;
use js_sys::Promise;
use nym_node_tester_utils::processor::Received;
use nym_node_tester_utils::receiver::ReceivedReceiver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::console_warn;

#[derive(Clone)]
pub(super) struct ReceivedReceiverWrapper(Arc<AsyncMutex<ReceivedReceiver<WasmTestMessageExt>>>);

impl ReceivedReceiverWrapper {
    pub(super) fn new(inner: ReceivedReceiver<WasmTestMessageExt>) -> Self {
        ReceivedReceiverWrapper(Arc::new(AsyncMutex::new(inner)))
    }

    pub(super) async fn clear_received_channel(&self) {
        let mut lost_msgs = 0;
        let mut lost_acks = 0;
        let mut permit = self.0.lock().await;
        while let Ok(Some(received)) = permit.try_next() {
            match received {
                Received::Message(_) => lost_msgs += 1,
                Received::Ack(_) => lost_acks += 1,
            }
        }
        if lost_msgs > 0 || lost_acks > 0 {
            console_warn!("while preparing for the test run, we cleared {lost_msgs} messages and {lost_acks} acks that were received in the meantime.")
        }
    }

    pub(super) async fn lock(&self) -> AsyncMutexGuard<'_, ReceivedReceiver<WasmTestMessageExt>> {
        self.0.lock().await
    }
}

pub(crate) struct TestMarker {
    value: Arc<AtomicBool>,
}

impl TestMarker {
    pub fn new(value: Arc<AtomicBool>) -> Self {
        Self { value }
    }
}

impl Drop for TestMarker {
    // make sure to clear the test flag when the marker is dropped
    fn drop(&mut self) {
        self.value.store(false, Ordering::SeqCst)
    }
}

pub(crate) trait GatewayReconnection {
    fn disconnect_from_gateway(&self) -> Promise;

    fn reconnect_to_gateway(&self) -> Promise;
}

impl GatewayReconnection for LockedGatewayClient {
    fn disconnect_from_gateway(&self) -> Promise {
        let this = self.clone();

        future_to_promise(async move {
            let mut guard = this.lock().await;
            guard
                .disconnect()
                .await
                .map_err(|err| JsValue::from(NodeTesterError::from(err)))?;
            Ok(JsValue::undefined())
        })
    }

    fn reconnect_to_gateway(&self) -> Promise {
        let this = self.clone();

        future_to_promise(async move {
            let mut guard = this.lock().await;
            guard
                .try_reconnect()
                .await
                .map_err(|err| JsValue::from(NodeTesterError::from(err)))?;
            Ok(JsValue::undefined())
        })
    }
}
