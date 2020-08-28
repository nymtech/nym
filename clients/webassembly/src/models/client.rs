// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
use crate::{console_log, console_error};
use crate::websocket::JSWebsocket;
use crypto::asymmetric::identity;
use directory_client::{DirectoryClient, Topology};
use gateway_requests::registration::handshake::{client_handshake, SharedKeys};
use js_sys::Promise;
use rand::rngs::OsRng;
use std::convert::TryInto;
use std::future::Future;
use topology::{gateway, NymTopology};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, spawn_local};

const DEFAULT_RNG: OsRng = OsRng;

struct GatewayClient {
    gateway_identity: identity::PublicKey,
    address: String,
    shared_key: Option<SharedKeys>,
    socket: Option<JSWebsocket>,
}

#[wasm_bindgen]
pub struct NymClient {
    version: String,
    directory_server: String,
    identity: identity::KeyPair,

    topology: Option<NymTopology>,
    gateway_client: Option<GatewayClient>,
}

#[wasm_bindgen]
impl NymClient {
    pub fn new(directory_server: String, version: String) -> Self {
        // for time being generate new identity each time...
        let identity = identity::KeyPair::new_with_rng(&mut DEFAULT_RNG);

        Self {
            identity,
            version,
            directory_server,
            topology: None,
            gateway_client: None,
        }
    }

    pub async fn initial_setup(mut self) -> Self {
        let mut client = self.get_and_update_topology().await;
        client.choose_gateway();
        client.connect_to_gateway();
        client.derive_shared_gateway_key().await;
        console_log!(
            "got shared key! {:?} (its id: {:?})",
            client.gateway_client.as_ref().unwrap().shared_key,
            client.gateway_client.as_ref().unwrap().gateway_identity
        );
        client
    }

    pub(crate) fn start_cover_traffic(&self) {
        spawn_local(async move { todo!("here be cover traffic") })
    }

    pub(crate) async fn derive_shared_gateway_key(&mut self) {
        let gateway_client = self.gateway_client.as_mut().unwrap();

        let mut gateway_socket = gateway_client
            .socket
            .as_mut()
            .expect("did not establish connection to the gateway!");
        let gateway_identity = gateway_client.gateway_identity;

        let shared_keys = client_handshake(
            &mut DEFAULT_RNG,
            &mut gateway_socket,
            &self.identity,
            gateway_identity,
        )
        .await;

        match shared_keys {
            Ok(keys) => gateway_client.shared_key = Some(keys),
            Err(err) => panic!("failed to perform gateway handshake! - {:?}", err),
        }
    }

    pub(crate) fn connect_to_gateway(&mut self) {
        let gateway_client = self.gateway_client.as_mut().unwrap();
        let gateway_address = gateway_client.address.as_ref();
        let gateway_socket =
            JSWebsocket::new(gateway_address).expect("failed to connect to the gateway");
        gateway_client.socket.replace(gateway_socket);
    }

    pub(crate) fn choose_gateway(&mut self) {
        let topology = self
            .topology
            .as_ref()
            .expect("did not obtain topology before");

        console_log!("topology: {:#?}", topology);

        // choose the first one available
        assert!(!topology.gateways().is_empty());
        let gateway = topology.gateways().first().unwrap();
        self.gateway_client = Some(GatewayClient {
            address: gateway.client_listener.clone(),
            gateway_identity: gateway.identity_key,
            shared_key: None,
            socket: None,
        })
    }

    // TODO: is it somehow possible to make it work with `&mut self`?
    pub async fn get_and_update_topology(mut self) -> Self {
        let new_topology = self.get_nym_topology().await;
        self.update_topology(new_topology);
        self
    }

    pub(crate) fn update_topology(&mut self, topology: NymTopology) {
        self.topology = Some(topology)
    }

    // #[wasm_bindgen(constructor)]
    // pub fn new() -> Self {
    //     ClientTest {
    //         version: "0.8".to_string(),
    //         directory_server: "http://localhost:8080".to_string(),
    //     }
    // }
    //
    // pub async fn do_foomp() -> String {
    // let topology = Self::get_topology().await;
    // format!("{:#?}", topology)

    // "aa".to_string()
    // spawn_local(async move { loop {} })
    // }

    pub fn get_full_topology_string(&self) -> Promise {
        let directory_client_config = directory_client::Config::new(self.directory_server.clone());
        let directory_client = directory_client::Client::new(directory_client_config);
        future_to_promise(async move {
            let string_topology =
                serde_json::to_string(&directory_client.get_topology().await.unwrap()).unwrap();
            Ok(JsValue::from(string_topology))
        })
    }

    pub(crate) async fn get_nym_topology(&self) -> NymTopology {
        let directory_client_config = directory_client::Config::new(self.directory_server.clone());
        let directory_client = directory_client::Client::new(directory_client_config);

        match directory_client.get_topology().await {
            Err(err) => panic!(err),
            Ok(topology) => {
                let nym_topology: NymTopology = topology
                    .try_into()
                    .ok()
                    .expect("this is not a NYM topology!");
                nym_topology.filter_system_version(&self.version)
            }
        }
    }
}
