// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::NODE_TESTER_ID;
use crate::error::WasmClientError;
use crate::helpers::{current_network_topology_async, setup_from_topology};
use crate::storage::ClientStorage;
use crate::tester::ephemeral_receiver::EphemeralTestReceiver;
use crate::tester::helpers::{
    GatewayReconnection, NodeTestResult, ReceivedReceiverWrapper, TestMarker, WasmTestMessageExt,
};
use crate::topology::WasmNymTopology;
use futures::channel::mpsc;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::key_manager::ManagedKeys;
use nym_client_core::init::InitialisationDetails;
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_gateway_client::GatewayClient;
use nym_node_tester_utils::receiver::SimpleMessageReceiver;
use nym_node_tester_utils::{NodeTester, TestMessage};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_sphinx::params::PacketSize;
use nym_sphinx::preparer::PreparedFragment;
use nym_task::TaskManager;
use nym_topology::NymTopology;
use nym_validator_client::client::IdentityKey;
use rand::rngs::OsRng;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex as SyncMutex};
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{check_promise_result, console_log, PromisableResult};

mod ephemeral_receiver;
pub(crate) mod helpers;

pub type NodeTestMessage = TestMessage<WasmTestMessageExt>;
type LockedGatewayClient =
    Arc<AsyncMutex<GatewayClient<FakeClient<DirectSigningNyxdClient>, EphemeralStorage>>>;

pub(crate) const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const DEFAULT_TEST_PACKETS: u32 = 20;

#[wasm_bindgen]
pub struct NymNodeTester {
    test_in_progress: Arc<AtomicBool>,

    // we need to increment the nonce between tests to distinguish the packets
    // but we can't make the tester mutable because of wasm...
    // so we're using the atomics
    current_test_nonce: AtomicU32,

    // blame all those mutexes on being unable to have an async method with internal mutability...
    tester: Arc<SyncMutex<NodeTester<OsRng>>>,
    gateway_client: LockedGatewayClient,

    // we have to put it behind the lock due to wasm limitations and borrowing...
    // the mutex acquisition should be instant as there aren't going to be any threads attempting
    // to get simultaneous access
    processed_receiver: ReceivedReceiverWrapper,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,
}

#[wasm_bindgen]
pub struct NymNodeTesterBuilder {
    gateway: Option<IdentityKey>,
    id: Option<String>,

    base_topology: NymTopology,

    // unimplemented
    bandwidth_controller:
        Option<BandwidthController<FakeClient<DirectSigningNyxdClient>, EphemeralStorage>>,
}

fn address(keys: &ManagedKeys, gateway_identity: NodeIdentity) -> Recipient {
    Recipient::new(
        *keys.identity_public_key(),
        *keys.encryption_public_key(),
        gateway_identity,
    )
}

#[wasm_bindgen]
impl NymNodeTesterBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        base_topology: WasmNymTopology,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> NymNodeTesterBuilder {
        NymNodeTesterBuilder {
            id,
            gateway,
            base_topology: base_topology.into(),
            bandwidth_controller: None,
        }
    }

    async fn _new_with_api(
        api_url: String,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> Result<Self, WasmClientError> {
        let topology = current_network_topology_async(api_url).await?;
        Ok(NymNodeTesterBuilder::new(topology, id, gateway))
    }

    pub fn new_with_api(
        api_url: String,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> Promise {
        future_to_promise(async move {
            Self::_new_with_api(api_url, id, gateway)
                .await
                .into_promise_result()
        })
    }

    async fn gateway_info(
        &self,
        client_store: &ClientStorage,
    ) -> Result<InitialisationDetails, WasmClientError> {
        if let Ok(loaded) = InitialisationDetails::try_load(client_store, client_store).await {
            Ok(loaded)
        } else {
            setup_from_topology(self.gateway.clone(), &self.base_topology, client_store).await
        }
    }

    async fn _setup_client(mut self) -> Result<NymNodeTester, WasmClientError> {
        let task_manager = TaskManager::default();

        let storage_id = if let Some(client_id) = &self.id {
            format!("{NODE_TESTER_ID}-{client_id}")
        } else {
            NODE_TESTER_ID.to_owned()
        };

        let client_store = ClientStorage::new_async(&storage_id, None).await?;

        let init_details = self.gateway_info(&client_store).await?;
        let gateway_endpoint = init_details.gateway_details;
        let gateway_identity = gateway_endpoint.try_get_gateway_identity_key()?;
        let managed_keys = init_details.managed_keys;

        let (mixnet_message_sender, mixnet_message_receiver) = mpsc::unbounded();
        let (ack_sender, ack_receiver) = mpsc::unbounded();

        let mut gateway_client = GatewayClient::new(
            gateway_endpoint.gateway_listener,
            managed_keys.identity_keypair(),
            gateway_identity,
            Some(managed_keys.must_get_gateway_shared_key()),
            mixnet_message_sender,
            ack_sender,
            Duration::from_secs(10),
            self.bandwidth_controller.take(),
            task_manager.subscribe(),
        );

        gateway_client.set_disabled_credentials_mode(true);
        gateway_client.authenticate_and_start().await?;

        // TODO: make those values configurable later
        let tester = NodeTester::new(
            OsRng,
            self.base_topology,
            Some(address(&managed_keys, gateway_identity)),
            PacketSize::default(),
            Duration::from_millis(5),
            Duration::from_millis(5),
            managed_keys.ack_key(),
        );

        let (processed_sender, processed_receiver) = mpsc::unbounded();

        let mut receiver = SimpleMessageReceiver::new_sphinx_receiver(
            managed_keys.encryption_keypair(),
            managed_keys.ack_key(),
            mixnet_message_receiver,
            ack_receiver,
            processed_sender,
            task_manager.subscribe(),
        );

        nym_task::spawn(async move { receiver.run().await });

        Ok(NymNodeTester {
            test_in_progress: Arc::new(AtomicBool::new(false)),
            current_test_nonce: Default::default(),
            tester: Arc::new(SyncMutex::new(tester)),
            gateway_client: Arc::new(AsyncMutex::new(gateway_client)),
            processed_receiver: ReceivedReceiverWrapper::new(processed_receiver),
            _task_manager: task_manager,
        })
    }

    pub fn setup_client(self) -> Promise {
        future_to_promise(async move { self._setup_client().await.into_promise_result() })
    }
}

async fn test_mixnode(
    test_packets: Vec<PreparedFragment>,
    gateway_client: LockedGatewayClient,
    processed_receiver: ReceivedReceiverWrapper,
    _test_marker: TestMarker,
    timeout: Duration,
) -> Result<NodeTestResult, WasmClientError> {
    let num_test_packets = test_packets.len() as u32;

    let expected_ack_ids = test_packets
        .iter()
        .map(|p| p.fragment_identifier)
        .collect::<HashSet<_>>();

    let mix_packets = test_packets.into_iter().map(|p| p.mix_packet).collect();

    // start by clearing any messages that might have been received between tests
    processed_receiver.clear_received_channel().await;

    // locking the gateway client so that we could get mutable access to data without having to declare
    // self mutable
    let mut gateway_permit = gateway_client.lock().await;
    gateway_permit.batch_send_mix_packets(mix_packets).await?;

    let receiver_permit = processed_receiver.lock().await;
    let result =
        EphemeralTestReceiver::new(num_test_packets, expected_ack_ids, receiver_permit, timeout)
            .perform_test()
            .await;

    Ok(result)
}

#[wasm_bindgen]
impl NymNodeTester {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        topology: WasmNymTopology,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> Promise {
        console_log!("constructing node tester!");
        NymNodeTesterBuilder::new(topology, id, gateway).setup_client()
    }

    async fn _new_with_api(
        api_url: String,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> Result<Self, WasmClientError> {
        NymNodeTesterBuilder::_new_with_api(api_url, id, gateway)
            .await?
            ._setup_client()
            .await
    }

    pub fn new_with_api(
        api_url: String,
        id: Option<String>,
        gateway: Option<IdentityKey>,
    ) -> Promise {
        future_to_promise(async move {
            Self::_new_with_api(api_url, id, gateway)
                .await
                .into_promise_result()
        })
    }

    pub fn disconnect_from_gateway(&self) -> Promise {
        self.gateway_client.disconnect_from_gateway()
    }

    pub fn reconnect_to_gateway(&self) -> Promise {
        self.gateway_client.reconnect_to_gateway()
    }

    fn prepare_test_packets(
        &self,
        mixnode_identity: String,
        test_nonce: u32,
        num_test_packets: u32,
    ) -> Result<Vec<PreparedFragment>, WasmClientError> {
        let test_ext = WasmTestMessageExt::new(test_nonce);
        let mut tester_permit = self.tester.lock().expect("mutex got poisoned");
        tester_permit
            .existing_identity_mixnode_test_packets(
                mixnode_identity,
                test_ext,
                num_test_packets,
                None,
            )
            .map_err(Into::into)
    }

    pub fn test_node(
        &self,
        mixnode_identity: String,
        timeout_millis: Option<u64>,
        num_test_packets: Option<u32>,
    ) -> Promise {
        // establish test parameters
        let timeout = timeout_millis
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_TEST_TIMEOUT);
        let num_test_packets = num_test_packets.unwrap_or(DEFAULT_TEST_PACKETS);

        // mark start of the test
        if self.test_in_progress.swap(true, Ordering::SeqCst) {
            return WasmClientError::TestInProgress.into_rejected_promise();
        }

        // prepare test packets
        // (I simultaneously feel both disgusted and amazed by this workaround)
        let test_nonce = self.current_test_nonce.fetch_add(1, Ordering::Relaxed);
        let test_packets = check_promise_result!(self.prepare_test_packets(
            mixnode_identity,
            test_nonce,
            num_test_packets
        ));

        let processed_receiver_clone = self.processed_receiver.clone();
        let gateway_client_clone = Arc::clone(&self.gateway_client);
        let tester_marker = TestMarker::new(Arc::clone(&self.test_in_progress));

        // start doing async things (send packets and watch for anything coming back)
        future_to_promise(async move {
            test_mixnode(
                test_packets,
                gateway_client_clone,
                processed_receiver_clone,
                tester_marker,
                timeout,
            )
            .await
            .into_promise_result()
        })
    }
}
