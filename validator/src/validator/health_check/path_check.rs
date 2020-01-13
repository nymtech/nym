use crypto::identity::{DummyMixIdentityKeyPair, MixnetIdentityKeyPair, MixnetIdentityPublicKey};
use itertools::Itertools;
use log::{debug, error, trace, warn};
use mix_client::MixClient;
use provider_client::ProviderClient;
use sphinx::header::delays::Delay;
use sphinx::route::{Destination, Node as SphinxNode};
use std::collections::HashMap;
use topology::MixProviderNode;

#[derive(Debug)]
pub(crate) enum PathCheckerError {
    CouldNotRegisterWithEndProviderError,
}

pub(crate) struct PathChecker {
    provider_clients: HashMap<[u8; 32], Option<ProviderClient>>,
    // currently this is an overkill as MixClient is extremely cheap to create,
    // however, once we introduce persistent connection between client and layer one mixes,
    // this will be extremely helfpul to have
    layer_one_clients: HashMap<[u8; 32], Option<MixClient>>,
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
        }
    }

    fn construct_check_message(path: &Vec<SphinxNode>) -> Vec<u8> {
        path.iter()
            .map(|node| node.pub_key.to_bytes().to_vec())
            .flatten()
            .collect()
    }

    pub(crate) async fn check_path(&mut self, path: &Vec<SphinxNode>) -> bool {
        debug!("Checking path: {:?}", path);

        let provider_client = self
            .provider_clients
            .get(&path.last().unwrap().pub_key.to_bytes())
            .unwrap();

        if provider_client.is_none() {
            debug!("we can ignore this path as provider itself is inaccessible");
            return false;
        }

        let provider_client = provider_client.as_ref().unwrap();

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
            return false;
        }

        let first_node_client = first_node_client.as_ref().unwrap();

        let packet_message = PathChecker::construct_check_message(path);
        let delays: Vec<_> = path.iter().map(|_| Delay::new(0)).collect();

        let packet =
            sphinx::SphinxPacket::new(packet_message, &path[..], &self.our_destination, &delays)
                .unwrap();

        debug!("sending test packet to {}", first_node_address);
        if first_node_client
            .send(packet, first_node_address)
            .await
            .is_err()
        {
            warn!("failed to send packet to {}", first_node_address);
            return false;
        }

        // TODO:
        true
    }
}
