// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on NodeTestResult
#![allow(clippy::drop_non_drop)]

use crate::error::WasmClientError;
use crate::tester::LockedGatewayClient;
use js_sys::Promise;
use nym_node_tester_utils::processor::Received;
use nym_node_tester_utils::receiver::ReceivedReceiver;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, console_warn};

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

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct WasmTestMessageExt {
    pub test_id: u32,
}

impl WasmTestMessageExt {
    pub fn new(test_id: u32) -> Self {
        WasmTestMessageExt { test_id }
    }
}

// TODO: maybe put it in the tester utils
#[wasm_bindgen]
pub struct NodeTestResult {
    pub sent_packets: u32,
    pub received_packets: u32,
    pub received_acks: u32,

    pub duplicate_packets: u32,
    pub duplicate_acks: u32,
}

impl Display for NodeTestResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test results: ")?;
        writeln!(f, "Total score: {:.2}%", self.score())?;
        writeln!(f, "Sent packets: {}", self.sent_packets)?;
        writeln!(f, "Received (valid) packets: {}", self.received_packets)?;
        writeln!(f, "Received (valid) acks: {}", self.received_acks)?;
        writeln!(f, "Received duplicate packets: {}", self.duplicate_packets)?;
        write!(f, "Received duplicate acks: {}", self.duplicate_acks)
    }
}

#[wasm_bindgen]
impl NodeTestResult {
    pub fn log_details(&self) {
        console_log!("{}", self)
    }

    pub fn score(&self) -> f32 {
        let expected = self.sent_packets * 2;
        let actual = (self.received_packets + self.received_acks)
            .saturating_sub(self.duplicate_packets + self.duplicate_acks);

        actual as f32 / expected as f32 * 100.
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
                .map_err(|err| JsValue::from(WasmClientError::from(err)))?;
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
                .map_err(|err| JsValue::from(WasmClientError::from(err)))?;
            Ok(JsValue::undefined())
        })
    }
}
