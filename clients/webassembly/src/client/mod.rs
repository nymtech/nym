// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use crate::client::helpers::{InputSender, NymClientTestRequest, WasmTopologyExt};
use crate::client::response_pusher::ResponsePusher;
use crate::constants::NODE_TESTER_CLIENT_ID;
use crate::error::WasmClientError;
use crate::helpers::{
    parse_recipient, parse_sender_tag, setup_from_topology, setup_gateway_from_api,
    setup_reply_surb_storage_backend,
};
use crate::storage::traits::FullWasmClientStorage;
use crate::storage::ClientStorage;
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState,
};
use nym_client_core::client::inbound_messages::InputMessage;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use nym_task::TaskManager;
use nym_topology::provider_trait::{HardcodedTopologyProvider, TopologyProvider};
use nym_topology::NymTopology;
use nym_validator_client::client::IdentityKey;
use rand::rngs::OsRng;
use rand::RngCore;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{check_promise_result, console_log, PromisableResult};

pub mod config;
mod helpers;
mod response_pusher;

#[wasm_bindgen]
pub struct NymClient {
    self_address: String,
    client_input: Arc<ClientInput>,
    client_state: Arc<ClientState>,

    // keep track of the "old" topology for the purposes of node tester
    // so that it could be restored after the check is done
    _full_topology: Option<NymTopology>,

    // even though we don't use graceful shutdowns, other components rely on existence of this struct
    // and if it's dropped, everything will start going offline
    _task_manager: TaskManager,

    packet_type: PacketType,
}

#[wasm_bindgen]
pub struct NymClientBuilder {
    config: Config,
    custom_topology: Option<NymTopology>,
    preferred_gateway: Option<IdentityKey>,

    storage_passphrase: Option<String>,
    on_message: js_sys::Function,
}

#[wasm_bindgen]
impl NymClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        config: Config,
        on_message: js_sys::Function,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Self {
        NymClientBuilder {
            config,
            custom_topology: None,
            storage_passphrase,
            on_message,
            preferred_gateway,
        }
    }

    // no cover traffic
    // no poisson delay
    // hardcoded topology
    // NOTE: you most likely want to use `[NymNodeTester]` instead.
    pub fn new_tester(
        topology: WasmNymTopology,
        on_message: js_sys::Function,
        gateway: Option<IdentityKey>,
    ) -> Self {
        if let Some(gateway_id) = &gateway {
            if !topology.ensure_contains_gateway_id(gateway_id) {
                panic!("the specified topology does not contain the gateway used by the client")
            }
        }

        let full_config = Config::new_tester_config(NODE_TESTER_CLIENT_ID);

        NymClientBuilder {
            config: full_config,
            custom_topology: Some(topology.into()),
            on_message,
            storage_passphrase: None,
            preferred_gateway: gateway,
        }
    }

    fn start_reconstructed_pusher(client_output: ClientOutput, on_message: js_sys::Function) {
        ResponsePusher::new(client_output, on_message).start()
    }

    fn topology_provider(&mut self) -> Option<Box<dyn TopologyProvider + Send + Sync>> {
        if let Some(hardcoded_topology) = self.custom_topology.take() {
            Some(Box::new(HardcodedTopologyProvider::new(hardcoded_topology)))
        } else {
            None
        }
    }

    fn initialise_storage(config: &Config, base_storage: ClientStorage) -> FullWasmClientStorage {
        FullWasmClientStorage {
            keys_and_gateway_store: base_storage,
            reply_storage: setup_reply_surb_storage_backend(config.base.debug.reply_surbs),
            credential_storage: EphemeralCredentialStorage::default(),
        }
    }

    async fn start_client_async(mut self) -> Result<NymClient, WasmClientError> {
        console_log!("Starting the wasm client");

        let nym_api_endpoints = self.config.base.client.nym_api_urls.clone();

        // TODO: this will have to be re-used for surbs. but this is a problem for another PR.
        let client_store =
            ClientStorage::new_async(&self.config.base.client.id, self.storage_passphrase.take())
                .await?;

        let user_chosen = self.preferred_gateway.clone();

        // if we provided hardcoded topology, get gateway from it, otherwise get it the 'standard' way
        if let Some(topology) = &self.custom_topology {
            setup_from_topology(user_chosen, topology, &client_store).await?
        } else {
            setup_gateway_from_api(&client_store, user_chosen, &nym_api_endpoints).await?
        };

        let packet_type = self.config.base.debug.traffic.packet_type;
        let storage = Self::initialise_storage(&self.config, client_store);
        let maybe_topology_provider = self.topology_provider();

        let mut base_builder: BaseClientBuilder<_, FullWasmClientStorage> =
            BaseClientBuilder::<FakeClient<DirectSigningNyxdClient>, _>::new(
                &self.config.base,
                storage,
                None,
            );
        if let Some(topology_provider) = maybe_topology_provider {
            base_builder = base_builder.with_topology_provider(topology_provider);
        }

        let mut started_client = base_builder.start_base().await?;
        let self_address = started_client.address.to_string();

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_reconstructed_pusher(client_output, self.on_message);

        Ok(NymClient {
            self_address,
            client_input: Arc::new(client_input),
            client_state: Arc::new(started_client.client_state),
            _full_topology: None,
            _task_manager: started_client.task_manager,
            packet_type,
        })
    }

    pub fn start_client(self) -> Promise {
        future_to_promise(async move { self.start_client_async().await.into_promise_result() })
    }
}

#[wasm_bindgen]
impl NymClient {
    async fn _new(
        config: Config,
        on_message: js_sys::Function,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Result<NymClient, WasmClientError> {
        NymClientBuilder::new(config, on_message, preferred_gateway, storage_passphrase)
            .start_client_async()
            .await
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: Config,
        on_message: js_sys::Function,
        preferred_gateway: Option<IdentityKey>,
        storage_passphrase: Option<String>,
    ) -> Promise {
        future_to_promise(async move {
            Self::_new(config, on_message, preferred_gateway, storage_passphrase)
                .await
                .into_promise_result()
        })
    }

    pub fn self_address(&self) -> String {
        self.self_address.clone()
    }

    pub fn try_construct_test_packet_request(
        &self,
        mixnode_identity: String,
        num_test_packets: Option<u32>,
    ) -> Promise {
        // TODO: improve the source of rng (i.e. don't make it ephemeral...)
        let mut ephemeral_rng = OsRng;
        let test_id = ephemeral_rng.next_u32();
        self.client_state
            .mix_test_request(test_id, mixnode_identity, num_test_packets)
    }

    pub fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise {
        self.client_state.change_hardcoded_topology(topology)
    }

    pub fn current_network_topology(&self) -> Promise {
        self.client_state.current_topology()
    }

    /// Sends a test packet through the current network topology.
    /// It's the responsibility of the caller to ensure the correct topology has been injected and
    /// correct onmessage handlers have been setup.
    pub fn try_send_test_packets(&mut self, request: NymClientTestRequest) -> Promise {
        // TOOD: use the premade packets instead
        console_log!(
            "Attempting to send {} test packets",
            request.test_msgs.len()
        );

        // our address MUST BE valid
        let recipient = parse_recipient(&self.self_address()).unwrap();

        let lane = TransmissionLane::General;
        let input_msgs = request
            .test_msgs
            .into_iter()
            .map(|p| InputMessage::new_regular(recipient, p, lane, None))
            .collect();

        self.client_input.send_messages(input_msgs)
    }

    /// The simplest message variant where no additional information is attached.
    /// You're simply sending your `data` to specified `recipient` without any tagging.
    ///
    /// Ends up with `NymMessage::Plain` variant
    pub fn send_regular_message(&self, message: Vec<u8>, recipient: String) -> Promise {
        console_log!(
            "Attempting to send {:.2} kiB message to {recipient}",
            message.len() as f64 / 1024.0
        );

        let recipient = check_promise_result!(parse_recipient(&recipient));

        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_regular(recipient, message, lane, Some(self.packet_type));
        self.client_input.send_message(input_msg)
    }

    /// Creates a message used for a duplex anonymous communication where the recipient
    /// will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
    ///
    /// Note that if reply_surbs is set to zero then
    /// this variant requires the client having sent some reply_surbs in the past
    /// (and thus the recipient also knowing our sender tag).
    ///
    /// Ends up with `NymMessage::Repliable` variant
    pub fn send_anonymous_message(
        &self,
        message: Vec<u8>,
        recipient: String,
        reply_surbs: u32,
    ) -> Promise {
        console_log!(
            "Attempting to anonymously send {:.2} kiB message to {recipient} while attaching {reply_surbs} replySURBs.",
            message.len() as f64 / 1024.0
        );

        let recipient = check_promise_result!(parse_recipient(&recipient));

        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_anonymous(
            recipient,
            message,
            reply_surbs,
            lane,
            Some(self.packet_type),
        );
        self.client_input.send_message(input_msg)
    }

    /// Attempt to use our internally received and stored `ReplySurb` to send the message back
    /// to specified recipient whilst not knowing its full identity (or even gateway).
    ///
    /// Ends up with `NymMessage::Reply` variant
    pub fn send_reply(&self, message: Vec<u8>, recipient_tag: String) -> Promise {
        console_log!(
            "Attempting to send {:.2} kiB reply message to {recipient_tag}",
            message.len() as f64 / 1024.0
        );

        let sender_tag = check_promise_result!(parse_sender_tag(&recipient_tag));

        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_reply(sender_tag, message, lane, Some(self.packet_type));
        self.client_input.send_message(input_msg)
    }
}
