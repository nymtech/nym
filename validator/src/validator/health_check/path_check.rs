use crypto::identity::{DummyMixIdentityKeyPair, MixnetIdentityKeyPair, MixnetIdentityPublicKey};
use log::{debug, error, trace, warn};
use provider_client::ProviderClient;
use sphinx::route::Node as SphinxNode;
use std::collections::HashMap;
use topology::MixProviderNode;

#[derive(Debug)]
pub(crate) enum PathCheckerError {
    CouldNotRegisterWithEndProviderError,
}

pub(crate) struct PathChecker {
    provider_clients: HashMap<[u8; 32], Option<ProviderClient>>,
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

            if insertion_result.is_none() {
                error!("provider {} already existed!", provider.pub_key);
            }
        }

        PathChecker { provider_clients }
    }

    pub(crate) fn check_path(&self, path: &Vec<SphinxNode>) -> bool {
        trace!("Checking path: {:?}", path);

        // TODO:
        true
    }
}
