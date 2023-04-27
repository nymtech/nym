// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use crate::client::helpers::{InputSender, NymClientTestRequest, WasmTopologyExt};
use crate::client::response_pusher::ResponsePusher;
use crate::error::WasmClientError;
use crate::helpers::{
    parse_recipient, parse_sender_tag, setup_new_key_manager, setup_reply_surb_storage_backend,
};
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::{
    BaseClientBuilder, ClientInput, ClientOutput, ClientState, CredentialsToggle,
};
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::client::{inbound_messages::InputMessage, key_manager::KeyManager};
use nym_client_core::config::{
    CoverTraffic, DebugConfig, GatewayEndpointConfig, Topology, Traffic,
};
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_sphinx::params::PacketType;
use nym_task::connections::TransmissionLane;
use nym_task::TaskManager;
use nym_topology::provider_trait::{HardcodedTopologyProvider, TopologyProvider};
use nym_topology::NymTopology;
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
    packet_type: Option<PacketType>,
}

#[wasm_bindgen]
pub struct NymClientBuilder {
    config: Config,
    custom_topology: Option<NymTopology>,

    /// KeyManager object containing smart pointers to all relevant keys used by the client.
    key_manager: KeyManager,

    reply_surb_storage_backend: browser_backend::Backend,

    on_message: js_sys::Function,

    // unimplemented:
    bandwidth_controller:
        Option<BandwidthController<FakeClient<DirectSigningNyxdClient>, EphemeralStorage>>,
    disabled_credentials: bool,
    packet_type: Option<PacketType>,
}

#[wasm_bindgen]
impl NymClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Config, on_message: js_sys::Function) -> Self {
        //, key_manager: Option<KeyManager>) {
        NymClientBuilder {
            reply_surb_storage_backend: setup_reply_surb_storage_backend(config.debug.reply_surbs),
            config,
            custom_topology: None,
            key_manager: setup_new_key_manager(),
            on_message,
            bandwidth_controller: None,
            disabled_credentials: true,
            packet_type: None,
        }
    }

    // no cover traffic
    // no poisson delay
    // hardcoded topology
    // NOTE: you most likely want to use `[NymNodeTester]` instead.
    pub fn new_tester(
        gateway_config: GatewayEndpointConfig,
        topology: WasmNymTopology,
        on_message: js_sys::Function,
    ) -> Self {
        if !topology.ensure_contains(&gateway_config) {
            panic!("the specified topology does not contain the gateway used by the client")
        }

        let full_config = Config {
            id: "ephemeral-id".to_string(),
            nym_api_url: None,
            disabled_credentials_mode: true,
            gateway_endpoint: gateway_config,
            debug: DebugConfig {
                traffic: Traffic {
                    disable_main_poisson_packet_distribution: true,
                    ..Default::default()
                },
                cover_traffic: CoverTraffic {
                    disable_loop_cover_traffic_stream: true,
                    ..Default::default()
                },
                topology: Topology {
                    disable_refreshing: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        NymClientBuilder {
            reply_surb_storage_backend: setup_reply_surb_storage_backend(
                full_config.debug.reply_surbs,
            ),
            config: full_config,
            custom_topology: Some(topology.into()),
            // TODO: once we make keys persistent, we'll require some kind of `init` method to generate
            // a prior shared keypair between the client and the gateway
            key_manager: setup_new_key_manager(),
            on_message,
            bandwidth_controller: None,
            disabled_credentials: true,
            packet_type: None,
        }
    }

    fn start_reconstructed_pusher(client_output: ClientOutput, on_message: js_sys::Function) {
        ResponsePusher::new(client_output, on_message).start()
    }

    fn topology_provider(&mut self) -> Option<Box<dyn TopologyProvider>> {
        if let Some(hardcoded_topology) = self.custom_topology.take() {
            Some(Box::new(HardcodedTopologyProvider::new(hardcoded_topology)))
        } else {
            None
        }
    }

    async fn start_client_async(mut self) -> Result<NymClient, WasmClientError> {
        console_log!("Starting the wasm client");

        let maybe_topology_provider = self.topology_provider();

        let disabled_credentials = if self.disabled_credentials {
            CredentialsToggle::Disabled
        } else {
            CredentialsToggle::Enabled
        };

        let nym_api_endpoints = match self.config.nym_api_url {
            Some(endpoint) => vec![endpoint],
            None => Vec::new(),
        };
        let mut base_builder = BaseClientBuilder::new(
            &self.config.gateway_endpoint,
            &self.config.debug,
            self.key_manager,
            self.bandwidth_controller,
            self.reply_surb_storage_backend,
            disabled_credentials,
            nym_api_endpoints,
        );
        if let Some(topology_provider) = maybe_topology_provider {
            base_builder = base_builder.with_topology_provider(topology_provider);
        }

        let self_address = base_builder.as_mix_recipient().to_string();
        let mut started_client = base_builder.start_base().await?;

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();

        Self::start_reconstructed_pusher(client_output, self.on_message);

        Ok(NymClient {
            self_address,
            client_input: Arc::new(client_input),
            client_state: Arc::new(started_client.client_state),
            _full_topology: None,
            _task_manager: started_client.task_manager,
            packet_type: self.packet_type,
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
    ) -> Result<NymClient, WasmClientError> {
        NymClientBuilder::new(config, on_message)
            .start_client_async()
            .await
    }

    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(config: Config, on_message: js_sys::Function) -> Promise {
        future_to_promise(async move { Self::_new(config, on_message).await.into_promise_result() })
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

        let input_msg = InputMessage::new_regular(recipient, message, lane, self.packet_type);
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

        let input_msg =
            InputMessage::new_anonymous(recipient, message, reply_surbs, lane, self.packet_type);
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

        let input_msg = InputMessage::new_reply(sender_tag, message, lane, self.packet_type);
        self.client_input.send_message(input_msg)
    }
}
