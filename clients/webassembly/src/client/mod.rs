// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::config::Config;
use crate::client::helpers::{InputSender, WasmTopologyExt};
use crate::client::response_pusher::ResponsePusher;
use crate::tester::NodeTesterRequest;
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
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_task::connections::TransmissionLane;
use nym_task::TaskManager;
use nym_topology::provider_trait::{HardcodedTopologyProvider, TopologyProvider};
use nym_topology::NymTopology;
use rand::rngs::OsRng;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_error, console_log};

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
}

#[wasm_bindgen]
impl NymClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Config, on_message: js_sys::Function) -> Self {
        //, key_manager: Option<KeyManager>) {
        NymClientBuilder {
            reply_surb_storage_backend: Self::setup_reply_surb_storage_backend(&config),
            config,
            custom_topology: None,
            key_manager: Self::setup_key_manager(),
            on_message,
            bandwidth_controller: None,
            disabled_credentials: true,
        }
    }

    // no cover traffic
    // no poisson delay
    // hardcoded topology
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
            disabled_credentials_mode: false,
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
            reply_surb_storage_backend: Self::setup_reply_surb_storage_backend(&full_config),
            config: full_config,
            custom_topology: Some(topology.into()),
            key_manager: Self::setup_key_manager(),
            on_message,
            bandwidth_controller: None,
            disabled_credentials: true,
        }
    }

    // TODO: once we make keys persistent, we'll require some kind of `init` method to generate
    // a prior shared keypair between the client and the gateway

    // perhaps this should be public?
    fn setup_key_manager() -> KeyManager {
        let mut rng = OsRng;
        // for time being generate new keys each time...
        console_log!("generated new set of keys");
        KeyManager::new(&mut rng)
    }

    // don't get too excited about the name, under the hood it's just a big fat placeholder
    // with no persistence
    fn setup_reply_surb_storage_backend(config: &Config) -> browser_backend::Backend {
        browser_backend::Backend::new(
            config
                .debug
                .reply_surbs
                .minimum_reply_surb_storage_threshold,
            config
                .debug
                .reply_surbs
                .maximum_reply_surb_storage_threshold,
        )
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

    pub async fn start_client(mut self) -> Promise {
        future_to_promise(async move {
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
            let mut started_client = match base_builder.start_base().await {
                Ok(base_client) => base_client,
                Err(err) => {
                    let error_msg = format!("failed to start the base client components - {err}");
                    console_error!("{}", error_msg);
                    let js_error = js_sys::Error::new(&error_msg);
                    return Err(JsValue::from(js_error));
                }
            };

            let client_input = started_client.client_input.register_producer();
            let client_output = started_client.client_output.register_consumer();

            Self::start_reconstructed_pusher(client_output, self.on_message);

            Ok(JsValue::from(NymClient {
                self_address,
                client_input: Arc::new(client_input),
                client_state: Arc::new(started_client.client_state),
                _full_topology: None,
                _task_manager: started_client.task_manager,
            }))
        })
    }
}

#[wasm_bindgen]
impl NymClient {
    pub fn self_address(&self) -> String {
        self.self_address.clone()
    }

    fn parse_recipient(recipient: &str) -> Result<Recipient, JsValue> {
        match Recipient::try_from_base58_string(recipient) {
            Ok(recipient) => Ok(recipient),
            Err(err) => {
                let error_msg = format!("{recipient} is not a valid Nym network recipient - {err}");
                console_error!("{}", error_msg);
                let js_error = js_sys::Error::new(&error_msg);
                Err(JsValue::from(js_error))
            }
        }
    }

    fn parse_sender_tag(tag: &str) -> Result<AnonymousSenderTag, JsValue> {
        match AnonymousSenderTag::try_from_base58_string(tag) {
            Ok(tag) => Ok(tag),
            Err(err) => {
                let error_msg = format!("{tag} is not a valid Nym AnonymousSenderTag - {err}");
                console_error!("{}", error_msg);
                let js_error = js_sys::Error::new(&error_msg);
                Err(JsValue::from(js_error))
            }
        }
    }

    pub fn try_construct_test_packet_request(&self, mixnode_identity: String) -> Promise {
        self.client_state.reduced_layer_topology(mixnode_identity)
    }

    pub fn try_send_test_packet(&mut self, request: NodeTesterRequest) -> Promise {
        todo!()

        // IDEAL PROCEDURE:
        // 1. check if mixnode exists
        // 2. get its layer L
        // 3. if possible, create ephemeral topology such that it that:
        //    - layer L only contains this one node
        //    - other layers contain only very high performance nodes
        // 4. send a sphinx packet through
        // 5. ???
        // 6 PROFIT

        // CURRENT (temporary?) PROCEDURE:
        // 1. check if mixnode exists
        // 2. get its layer L
        // 3. clear other nodes on layer L (other layers are not really guaranteed to be 'good')
        // 4. send a sphinx packet through
        // 5. ???
        // 6 PROFIT

        // check if this mixnode exists in our known topology and if so,
        // extract its layer
        // let layer_promise = self
        //     .client_state
        //     .check_for_mixnode_existence(mixnode_identity);
        //
        // let send_callback = Closure::new(|layer_res| {
        //     console_log!("RES: {layer_res:?}");
        // });
        //
        // let send_promise = layer_promise.then(&send_callback);
        // send_callback.forget();
        // send_promise
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

        let recipient = match Self::parse_recipient(&recipient) {
            Ok(recipient) => recipient,
            Err(err) => return Promise::reject(&err),
        };
        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_regular(recipient, message, lane);
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

        let recipient = match Self::parse_recipient(&recipient) {
            Ok(recipient) => recipient,
            Err(err) => return Promise::reject(&err),
        };
        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_anonymous(recipient, message, reply_surbs, lane);
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

        let sender_tag = match Self::parse_sender_tag(&recipient_tag) {
            Ok(recipient) => recipient,
            Err(err) => return Promise::reject(&err),
        };
        let lane = TransmissionLane::General;

        let input_msg = InputMessage::new_reply(sender_tag, message, lane);
        self.client_input.send_message(input_msg)
    }

    pub fn change_hardcoded_topology(&self, topology: WasmNymTopology) -> Promise {
        self.client_state.change_hardcoded_topology(topology)
    }
}
