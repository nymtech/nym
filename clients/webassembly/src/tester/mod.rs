// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::helpers::setup_new_key_manager;
use crate::topology::WasmNymTopology;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use futures::StreamExt;
use js_sys::Promise;
use node_tester_utils::receiver::{Received, ReceivedReceiver, SimpleMessageReceiver};
use node_tester_utils::{Empty, NodeTester, TestMessage};
use nym_client_core::client::key_manager::KeyManager;
use nym_client_core::config::GatewayEndpointConfig;
use nym_crypto::asymmetric::identity;
use nym_gateway_client::bandwidth::BandwidthController;
use nym_gateway_client::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_gateway_client::{AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_sphinx::params::PacketSize;
use nym_task::{TaskClient, TaskManager};
use nym_topology::NymTopology;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as SyncMutex};
use std::time::Duration;
use tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};
use wasm_utils::{console_log, console_warn};

// TODO: split this file

const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_TEST_PACKETS: u32 = 20;

#[derive(Clone)]
struct ReceivedReceiverWrapper(Arc<AsyncMutex<ReceivedReceiver>>);

impl ReceivedReceiverWrapper {
    fn new(inner: ReceivedReceiver) -> Self {
        ReceivedReceiverWrapper(Arc::new(AsyncMutex::new(inner)))
    }

    async fn clear_received_channel(&self) {
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

    async fn lock(&self) -> AsyncMutexGuard<'_, ReceivedReceiver> {
        self.0.lock().await
    }
}

// type TesterCommandReceiver = mpsc::UnboundedReceiver<TesterCommand>;
// struct TesterCommandSender(mpsc::UnboundedSender<TesterCommand>);
//
// impl From<mpsc::UnboundedSender<TesterCommand>> for TesterCommandSender {
//     fn from(value: UnboundedSender<TesterCommand>) -> Self {
//         TesterCommandSender(value)
//     }
// }
//
// impl TesterCommandSender {
//     fn send_command(&self, cmd: TesterCommand) {
//         self.0
//             .unbounded_send(cmd)
//             .expect("The TesterCommandReceiver has stopped receiving - the node tester is dead")
//     }
// }
//
// struct TesterCommand {
//     test_nonce: u32,
//     mixnode_identity: String,
//     test_packets: u32,
// }
//
// impl TesterCommand {
//     fn new(test_nonce: u32, mixnode_identity: String, test_packets: u32) -> Self {
//         Self {
//             test_nonce,
//             mixnode_identity,
//             test_packets,
//         }
//     }
// }

#[derive(Serialize, Deserialize)]
pub struct TestMessageExt {
    pub test_id: u32,
}

impl TestMessageExt {
    pub fn new(test_id: u32) -> Self {
        TestMessageExt { test_id }
    }
}

#[wasm_bindgen]
pub struct NodeTesterRequest {
    pub(crate) test_msg: TestMessage<TestMessageExt>,

    // specially constructed network topology that only contains the target
    // node on the tested layer
    pub(crate) testable_topology: NymTopology,
}

#[wasm_bindgen]
impl NodeTesterRequest {
    pub fn injectable_topology(&self) -> WasmNymTopology {
        self.testable_topology.clone().into()
    }
}

// TODO: maybe put it in the tester utils
#[wasm_bindgen]
pub struct NodeTestResult {
    pub sent_packets: u32,
    pub received_packets: u32,
    pub received_acks: u32,
}

#[wasm_bindgen]
impl NodeTestResult {
    pub fn score(&self) -> f32 {
        (self.received_packets + self.received_acks) as f32 / (self.sent_packets * 2) as f32 * 100.
    }
}

// // we need to keep this guy in separate task so that we wouldn't need to mutate our NymNodeTester
// // (gotta love async wasm)
// struct NodeTesterWrapper<R> {
//     tester: NodeTester<R>,
//     gateway_client: GatewayClient<FakeClient<DirectSigningNyxdClient>>,
//     command_receiver: TesterCommandReceiver,
//     shutdown: TaskClient,
// }
//
// impl<R> NodeTesterWrapper<R> {
//     fn new(
//         tester: NodeTester<R>,
//         gateway_client: GatewayClient<FakeClient<DirectSigningNyxdClient>>,
//         command_receiver: TesterCommandReceiver,
//         shutdown: TaskClient,
//     ) -> Self {
//         NodeTesterWrapper {
//             tester,
//             gateway_client,
//             command_receiver,
//             shutdown,
//         }
//     }
//
//     fn on_command(&mut self, command: TesterCommand) {
//         todo!()
//     }
//
//     async fn run(&mut self) {
//         while !self.shutdown.is_shutdown() {
//             tokio::select! {
//                 biased;
//                 _ = self.shutdown.recv() => {
//                     todo!()
//                 }
//                 cmd = self.command_receiver.next() => {
//                     let Some(cmd) = cmd else {
//                         todo!()
//                     };
//                     self.on_command(cmd)
//                 }
//             }
//         }
//     }
// }

// struct TestingInner<R> {
//     tester: NodeTester<R>,
//     gateway_client: GatewayClient<FakeClient<DirectSigningNyxdClient>>,
// }

#[wasm_bindgen]
pub struct NymNodeTester {
    // we need to increment the nonce between tests to distinguish the packets
    // but we can't make the tester mutable because of wasm...
    // so we're using the atomics
    current_test_nonce: AtomicU32,

    // blame all those mutexes on being unable to have an async method with internal mutability...
    tester: Arc<SyncMutex<NodeTester<OsRng>>>,
    gateway_client: Arc<AsyncMutex<GatewayClient<FakeClient<DirectSigningNyxdClient>>>>,

    // tester_command_sender: TesterCommandSender,

    // we have to put it behind the lock due to wasm limitations and borrowing...
    // the mutex acquisition should be instant as there aren't going to be any threads attempting
    // to get simultaneous access
    processed_receiver: ReceivedReceiverWrapper,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,
    // TODO: do we need any callbacks here?
}

#[wasm_bindgen]
pub struct NymNodeTesterBuilder {
    gateway_config: GatewayEndpointConfig,

    base_topology: NymTopology,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,

    bandwidth_controller: Option<BandwidthController<FakeClient<DirectSigningNyxdClient>>>,
}

fn address(keys: &KeyManager, gateway_identity: NodeIdentity) -> Recipient {
    Recipient::new(
        *keys.identity_keypair().public_key(),
        *keys.encryption_keypair().public_key(),
        gateway_identity,
    )
}

#[wasm_bindgen]
impl NymNodeTesterBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        gateway_config: GatewayEndpointConfig,
        base_topology: WasmNymTopology,
    ) -> NymNodeTesterBuilder {
        NymNodeTesterBuilder {
            gateway_config,
            base_topology: base_topology.into(),
            key_manager: setup_new_key_manager(),
            bandwidth_controller: None,
        }
    }

    pub fn new_with_api(
        gateway_config: GatewayEndpointConfig,
        api_url: String,
    ) -> NymNodeTesterBuilder {
        todo!()
    }

    async fn _setup_client(mut self) -> Result<NymNodeTester, WasmClientError> {
        let rng = OsRng;
        let task_manager = TaskManager::default();

        let gateway_identity =
            identity::PublicKey::from_base58_string(self.gateway_config.gateway_id)
                .map_err(|source| WasmClientError::InvalidGatewayIdentity { source })?;

        // we **REALLY** need persistence...
        let shared_key = if self.key_manager.is_gateway_key_set() {
            Some(self.key_manager.gateway_shared_key())
        } else {
            console_warn!("Gateway key not set - will derive a fresh one.");
            None
        };

        let (mixnet_message_sender, mixnet_message_receiver) = mpsc::unbounded();
        let (ack_sender, ack_receiver) = mpsc::unbounded();

        let mut gateway_client = GatewayClient::new(
            self.gateway_config.gateway_listener,
            self.key_manager.identity_keypair(),
            gateway_identity,
            shared_key,
            mixnet_message_sender,
            ack_sender,
            Duration::from_secs(10),
            self.bandwidth_controller.take(),
            task_manager.subscribe(),
        );

        gateway_client.set_disabled_credentials_mode(true);
        let shared_keys = gateway_client.authenticate_and_start().await?;

        // currently pointless but might as well do it for the future ¯\_(ツ)_/¯
        self.key_manager.insert_gateway_shared_key(shared_keys);

        // TODO: make those values configurable later
        let tester = NodeTester::new(
            rng,
            self.base_topology,
            address(&self.key_manager, gateway_identity),
            PacketSize::default(),
            Duration::from_millis(5),
            Duration::from_millis(5),
            self.key_manager.ack_key(),
        );

        let (processed_sender, processed_receiver) = mpsc::unbounded();

        let mut receiver = SimpleMessageReceiver::new_sphinx_receiver(
            self.key_manager.encryption_keypair(),
            self.key_manager.ack_key(),
            mixnet_message_receiver,
            ack_receiver,
            processed_sender,
            task_manager.subscribe(),
        );

        nym_task::spawn(async move { receiver.run().await });

        Ok(NymNodeTester {
            current_test_nonce: Default::default(),
            // tester_command_sender: tester_command_sender.into(),
            // testing: Arc::new(Mutex::new(TestingInner {
            //     tester,
            //     gateway_client,
            // })),
            tester: Arc::new(SyncMutex::new(tester)),
            gateway_client: Arc::new(AsyncMutex::new(gateway_client)),
            processed_receiver: ReceivedReceiverWrapper::new(processed_receiver),
            _task_manager: task_manager,
        })
    }

    pub fn setup_client(self) -> Promise {
        future_to_promise(async move {
            match self._setup_client().await {
                Ok(client) => Ok(JsValue::from(client)),
                Err(err) => Err(err.into()),
            }
        })
    }
}

#[wasm_bindgen]
impl NymNodeTester {
    #[wasm_bindgen(constructor)]
    // TODO: see if this constructor actually works...
    #[allow(clippy::new_ret_no_self)]
    pub fn new(gateway_config: GatewayEndpointConfig, topology: WasmNymTopology) -> Promise {
        console_log!("constructing node tester!");
        NymNodeTesterBuilder::new(gateway_config, topology).setup_client()
    }

    pub fn new_with_api() {
        //
    }

    pub fn test_node(
        &self,
        mixnode_identity: String,
        timeout_millis: Option<u64>,
        num_test_packets: Option<u32>,
    ) -> Promise {
        let timeout = timeout_millis
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_TEST_TIMEOUT);

        let num_test_packets = num_test_packets.unwrap_or(DEFAULT_TEST_PACKETS);

        // I simultaneously feel both disgusted and amazed by this workaround
        let test_nonce = self.current_test_nonce.fetch_add(1, Ordering::Relaxed);

        // let new_test = TesterCommand::new(test_nonce, mixnode_identity, test_packets);
        // self.tester_command_sender.send_command(new_test);

        let test_ext = TestMessageExt::new(test_nonce);

        let mut tester_permit = self.tester.lock().expect("mutex got poisoned");
        let test_packets = match tester_permit.existing_identity_mixnode_test_packets(
            mixnode_identity,
            test_ext,
            num_test_packets,
        ) {
            Ok(packets) => packets,
            Err(err) => return Promise::reject(&WasmClientError::from(err).into()),
        };

        let expected_ack_ids = test_packets
            .iter()
            .map(|p| p.fragment_identifier)
            .collect::<HashSet<_>>();

        let mix_packets = test_packets.into_iter().map(|p| p.mix_packet).collect();

        let processed_receiver_clone = self.processed_receiver.clone();
        let gateway_client_clone = Arc::clone(&self.gateway_client);

        future_to_promise(async move {
            processed_receiver_clone.clear_received_channel().await;

            let mut gateway_permit = gateway_client_clone.lock().await;
            if let Err(err) = gateway_permit.batch_send_mix_packets(mix_packets).await {
                return Err(WasmClientError::from(err).into());
            }

            let mut received_valid_messages = 0;
            let mut received_valid_acks = 0;

            let mut timeout_fut = wasm_timer::Delay::new(timeout);
            let mut receiver_permit = processed_receiver_clone.lock().await;

            loop {
                tokio::select! {
                    _ = &mut timeout_fut => {
                        break
                    }
                    received_packet = receiver_permit.next() => {
                        let Some(received_packet) = received_packet else {
                            todo!()
                        };
                        match received_packet {
                            Received::Message(msg) => {
                                console_log!("received msg! raw: {msg}");
                                let inner = msg.into_inner_data();
                                let foo = String::from_utf8_lossy(&inner);
                                console_log!("inner: {foo}");
                                // TODO: parsing etc
                                received_valid_messages += 1;
                            },
                            Received::Ack(frag_id) => {
                                console_log!("received ack! raw: {frag_id}");

                                if expected_ack_ids.contains(&frag_id) {
                                    received_valid_acks += 1;
                                } else {
                                    console_warn!("received an ack that was not part of the test! (id: {frag_id})")
                                }
                            }
                        }
                    }
                }
            }

            Ok(JsValue::from(NodeTestResult {
                sent_packets: num_test_packets,
                received_packets: received_valid_messages,
                received_acks: received_valid_acks,
            }))
        })
    }
}
