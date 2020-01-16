use crypto::identity::{DummyMixIdentityKeyPair, MixnetIdentityKeyPair, MixnetIdentityPublicKey};
use itertools::Itertools;
use log::{debug, error, warn};
use mix_client::MixClient;
use provider_client::ProviderClient;
use sphinx::header::delays::Delay;
use sphinx::route::{Destination, Node as SphinxNode};
use std::collections::HashMap;
use topology::MixProviderNode;

#[derive(Debug, PartialEq, Clone)]
pub enum PathStatus {
    Healthy,
    Unhealthy,
    Pending,
}

pub(crate) struct PathChecker {
    provider_clients: HashMap<[u8; 32], Option<ProviderClient>>,
    // currently this is an overkill as MixClient is extremely cheap to create,
    // however, once we introduce persistent connection between client and layer one mixes,
    // this will be extremely helpful to have
    layer_one_clients: HashMap<[u8; 32], Option<MixClient>>,
    paths_status: HashMap<Vec<u8>, PathStatus>,
    our_destination: Destination,
}

impl PathChecker {
    pub(crate) async fn new(
        providers: Vec<MixProviderNode>,
        ephemeral_keys: DummyMixIdentityKeyPair,
    ) -> Self {
        let mut provider_clients = HashMap::new();

        let mut temporary_address = [0u8; 32];
        let public_key_bytes = ephemeral_keys.public_key().to_bytes();
        temporary_address.copy_from_slice(&public_key_bytes[..]);

        for provider in providers {
            let mut provider_client =
                ProviderClient::new(provider.client_listener, temporary_address, None);
            let insertion_result = match provider_client.register().await {
                Ok(token) => {
                    debug!("registered at provider {}", provider.pub_key);
                    provider_client.update_token(token);
                    provider_clients.insert(provider.get_pub_key_bytes(), Some(provider_client))
                }
                Err(err) => {
                    warn!(
                        "failed to register at provider {} - {:?}",
                        provider.pub_key, err
                    );
                    provider_clients.insert(provider.get_pub_key_bytes(), None)
                }
            };

            if insertion_result.is_some() {
                error!("provider {} already existed!", provider.pub_key);
            }
        }

        PathChecker {
            provider_clients,
            layer_one_clients: HashMap::new(),
            our_destination: Destination::new(temporary_address, Default::default()),
            paths_status: HashMap::new(),
        }
    }

    // iteration is used to distinguish packets sent through the same path (as the healthcheck
    // may try to send say 10 packets through given path)
    fn unique_path_key(path: &Vec<SphinxNode>, iteration: u8) -> Vec<u8> {
        std::iter::once(iteration)
            .chain(
                path.iter()
                    .map(|node| node.pub_key.to_bytes().to_vec())
                    .flatten(),
            )
            .collect()
    }

    pub(crate) fn path_key_to_node_keys(path_key: Vec<u8>) -> Vec<[u8; 32]> {
        assert_eq!(path_key.len() % 32, 1);
        path_key
            .into_iter()
            .skip(1) // remove first byte as it represents the iteration number which we do not care about now
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
        &self,
        provider_client: &ProviderClient,
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
                        if msg == sfw_provider_requests::DUMMY_MESSAGE_CONTENT {
                            // finish iterating the loop as the messages might not be ordered
                            should_stop = true;
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
        for provider_client in self.provider_clients.values() {
            // if it was none all associated paths were already marked as unhealthy
            if provider_client.is_some() {
                let pc = provider_client.as_ref().unwrap();
                provider_messages.extend(self.resolve_pending_provider_checks(pc).await);
            }
        }

        self.update_path_statuses(provider_messages);
    }

    pub(crate) async fn send_test_packet(&mut self, path: &Vec<SphinxNode>, iteration: u8) {
        debug!("Checking path: {:?} ({})", path, iteration);
        let path_identifier = PathChecker::unique_path_key(path, iteration);

        // check if there is even any point in sending the packet

        // does provider exist?
        let provider_client = self
            .provider_clients
            .get(&path.last().unwrap().pub_key.to_bytes())
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

        let layer_one_mix = path.first().unwrap();
        let first_node_key = layer_one_mix.pub_key.to_bytes();
        let first_node_address =
            addressing::socket_address_from_encoded_bytes(layer_one_mix.address.to_bytes());

        let first_node_client = self
            .layer_one_clients
            .entry(first_node_key)
            .or_insert(Some(mix_client::MixClient::new()));

        if first_node_client.is_none() {
            debug!("we can ignore this path as layer one mix is inaccessible");
            if self
                .paths_status
                .insert(path_identifier, PathStatus::Unhealthy)
                .is_some()
            {
                panic!("Overwriting path checks!")
            }
            return;
        }

        let first_node_client = first_node_client.as_ref().unwrap();

        let delays: Vec<_> = path.iter().map(|_| Delay::new(0)).collect();

        let packet = sphinx::SphinxPacket::new(
            path_identifier.clone(),
            &path[..],
            &self.our_destination,
            &delays,
        )
        .unwrap();

        debug!("sending test packet to {}", first_node_address);
        match first_node_client.send(packet, first_node_address).await {
            Err(err) => {
                warn!("failed to send packet to {} - {}", first_node_address, err);
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
