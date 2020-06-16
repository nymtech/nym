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

use crate::MAX_PROVIDER_RESPONSE_SIZE;
use crypto::identity::MixIdentityKeyPair;
use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::{Delay, Destination, DestinationAddressBytes, Node as SphinxNode};
// use provider_client::{ProviderClient, ProviderClientError};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time::Duration;
use topology::provider;

pub(crate) type CheckId = [u8; 16];

// JUST TO MAKE IT COMPILE, THIS WILL NEED TO BE REPLACED WITH GATEWAY
struct ProviderClient {}

struct AuthToken {}

impl ProviderClient {
    fn new(
        _provider_network_address: SocketAddr,
        _our_address: DestinationAddressBytes,
        _auth_token: Option<AuthToken>,
        _max_response_size: usize,
    ) -> Self {
        unimplemented!()
    }
    fn update_token(&mut self, _auth_token: AuthToken) {
        unimplemented!()
    }
    async fn register(&mut self) -> Result<AuthToken, ProviderClientError> {
        unimplemented!()
    }
    async fn retrieve_messages(&mut self) -> Result<Vec<Vec<u8>>, ProviderClientError> {
        unimplemented!()
    }
}

#[derive(Debug)]
enum ProviderClientError {
    #[allow(dead_code)]
    ClientAlreadyRegisteredError,
}

const DUMMY_MESSAGE_CONTENT: &[u8] = b"THIS NEEDS TO BE REDONE AS IT DOESNT EXIST ANYMORE";

#[derive(Debug, PartialEq, Clone)]
pub enum PathStatus {
    Healthy,
    Unhealthy,
    Pending,
}

pub(crate) struct PathChecker {
    provider_clients: HashMap<[u8; 32], Option<ProviderClient>>,
    mixnet_client: mixnet_client::Client,
    paths_status: HashMap<Vec<u8>, PathStatus>,
    our_destination: Destination,
    check_id: CheckId,
}

impl PathChecker {
    pub(crate) async fn new(
        providers: Vec<provider::Node>,
        identity_keys: &MixIdentityKeyPair,
        connection_timeout: Duration,
        check_id: CheckId,
    ) -> Self {
        let mut provider_clients = HashMap::new();

        let address = identity_keys.public_key().derive_address();

        for provider in providers {
            let mut provider_client = ProviderClient::new(
                provider.client_listener,
                address.clone(),
                None,
                MAX_PROVIDER_RESPONSE_SIZE,
            );
            // TODO: we might be sending unnecessary register requests since after first healthcheck,
            // we are registered for any subsequent ones (since our address did not change)

            let insertion_result = match provider_client.register().await {
                Ok(token) => {
                    debug!("[Healthcheck] registered at provider {}", provider.pub_key);
                    provider_client.update_token(token);
                    provider_clients.insert(provider.get_pub_key_bytes(), Some(provider_client))
                }
                Err(ProviderClientError::ClientAlreadyRegisteredError) => {
                    info!("[Healthcheck] We were already registered");
                    provider_clients.insert(provider.get_pub_key_bytes(), Some(provider_client))
                }
            };

            if insertion_result.is_some() {
                error!("provider {} already existed!", provider.pub_key);
            }
        }

        // there's no reconnection allowed - if it fails, then it fails.
        let mixnet_client_config = mixnet_client::Config::new(
            Duration::from_secs(1_000_000_000),
            Duration::from_secs(1_000_000_000),
            connection_timeout,
        );

        PathChecker {
            provider_clients,
            mixnet_client: mixnet_client::Client::new(mixnet_client_config),
            our_destination: Destination::new(address, Default::default()),
            paths_status: HashMap::new(),
            check_id,
        }
    }

    // iteration is used to distinguish packets sent through the same path (as the healthcheck
    // may try to send say 10 packets through given path)
    fn unique_path_key(path: &[SphinxNode], check_id: CheckId, iteration: u8) -> Vec<u8> {
        check_id
            .iter()
            .cloned()
            .chain(std::iter::once(iteration))
            .chain(
                path.iter()
                    .map(|node| node.pub_key.to_bytes().to_vec())
                    .flatten(),
            )
            .collect()
    }

    pub(crate) fn path_key_to_node_keys(path_key: Vec<u8>) -> Vec<[u8; 32]> {
        assert_eq!(path_key.len() % 32, 17);
        path_key
            .into_iter()
            .skip(16 + 1) // remove 16 + 1 bytes as it represents check_id and the iteration number which we do not care about now
            .chunks(32)
            .into_iter()
            .map(|key_chunk| {
                let key_chunk_vec: Vec<_> = key_chunk.collect();
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_chunk_vec);
                key
            })
            .collect()
    }

    fn update_path_statuses(&mut self, messages: Vec<Vec<u8>>) {
        for msg in messages.into_iter() {
            // mark path as healthy
            let previous_status = self.paths_status.insert(msg, PathStatus::Healthy);
            match previous_status {
                None => warn!("we received information about unknown path! - perhaps somebody is messing with healthchecker?"),
                Some(status) => {
                    if status != PathStatus::Pending {
                        warn!("we received information about path that WASN'T in PENDING state! (it was in {:?}", status);
                    }
                }
            }
        }
    }

    // consume path_checker and return all path statuses
    pub(crate) fn get_all_statuses(self) -> HashMap<Vec<u8>, PathStatus> {
        self.paths_status
    }

    // pull messages from given provider until there are no more 'real' messages
    async fn resolve_pending_provider_checks(
        provider_client: &mut ProviderClient,
        check_id: CheckId,
    ) -> Vec<Vec<u8>> {
        // keep getting messages until we encounter the dummy message
        let mut provider_messages = Vec::new();
        loop {
            match provider_client.retrieve_messages().await {
                Err(err) => {
                    error!("failed to fetch provider messages! - {:?}", err);
                    break;
                }
                Ok(messages) => {
                    let mut should_stop = false;
                    for msg in messages.into_iter() {
                        trace!("received provider response: {:?}", msg);
                        if msg == DUMMY_MESSAGE_CONTENT {
                            // finish iterating the loop as the messages might not be ordered
                            should_stop = true;
                        } else if msg[..16] != check_id {
                            warn!("received response from previous healthcheck")
                        } else {
                            provider_messages.push(msg);
                        }
                    }
                    if should_stop {
                        break;
                    }
                }
            }
        }
        provider_messages
    }

    pub(crate) async fn resolve_pending_checks(&mut self) {
        // not sure how to nicely put it into an iterator due to it being async calls
        let mut provider_messages = Vec::new();
        for provider_client in self.provider_clients.values_mut() {
            // if it was none all associated paths were already marked as unhealthy
            let pc = match provider_client {
                Some(pc) => pc,
                None => continue,
            };

            provider_messages
                .extend(Self::resolve_pending_provider_checks(pc, self.check_id).await);
        }

        self.update_path_statuses(provider_messages);
    }

    pub(crate) async fn send_test_packet(&mut self, path: &[SphinxNode], iteration: u8) {
        if path.is_empty() {
            warn!("trying to send test packet through an empty path!");
            return;
        }

        trace!("Checking path: {:?} ({})", path, iteration);
        let path_identifier = PathChecker::unique_path_key(path, self.check_id, iteration);

        // check if there is even any point in sending the packet

        // does provider exist?
        let provider_client = self
            .provider_clients
            .get(
                &path
                    .last()
                    .expect("We checked the path to contain at least one entry")
                    .pub_key
                    .to_bytes(),
            )
            .unwrap();

        if provider_client.is_none() {
            debug!("we can ignore this path as provider itself is inaccessible");
            if self
                .paths_status
                .insert(path_identifier, PathStatus::Unhealthy)
                .is_some()
            {
                panic!("Overwriting path checks!")
            }
            return;
        }

        let layer_one_mix = path
            .first()
            .expect("We checked the path to contain at least one entry");

        // we generated the bytes data so unwrap is fine
        let first_node_address: SocketAddr =
            NymNodeRoutingAddress::try_from(layer_one_mix.address.clone())
                .unwrap()
                .into();

        let delays: Vec<_> = path.iter().map(|_| Delay::new_from_nanos(0)).collect();

        // all of the data used to create the packet was created by us
        let packet = nymsphinx::SphinxPacket::new(
            path_identifier.clone(),
            &path[..],
            &self.our_destination,
            &delays,
        )
        .unwrap();

        debug!("sending test packet to {}", first_node_address);

        match self
            .mixnet_client
            .send(first_node_address, packet, true)
            .await
        {
            Err(err) => {
                debug!("failed to send packet to {} - {}", first_node_address, err);
                if self
                    .paths_status
                    .insert(path_identifier, PathStatus::Unhealthy)
                    .is_some()
                {
                    panic!("Overwriting path checks!")
                }
            }
            Ok(_) => {
                if self
                    .paths_status
                    .insert(path_identifier, PathStatus::Pending)
                    .is_some()
                {
                    panic!("Overwriting path checks!")
                }
            }
        }
    }
}
