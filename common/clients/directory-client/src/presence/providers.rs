use super::PresenceEq;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use topology::provider;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderPresence {
    pub client_listener: String,
    pub mixnet_listener: String,
    pub pub_key: String,
    pub registered_clients: Vec<MixProviderClient>,
    pub last_seen: u64,
    pub version: String,
}

impl PresenceEq for MixProviderPresence {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.registered_clients.len() != other.registered_clients.len() {
            return false;
        }

        if self.client_listener != other.client_listener
            || self.mixnet_listener != other.mixnet_listener
            || self.pub_key != other.pub_key
            || self.version != other.version
        {
            return false;
        }

        let clients_self_set: HashSet<_> =
            self.registered_clients.iter().map(|c| &c.pub_key).collect();
        let clients_other_set: HashSet<_> = other
            .registered_clients
            .iter()
            .map(|c| &c.pub_key)
            .collect();

        clients_self_set == clients_other_set
    }
}

impl Into<topology::provider::Node> for MixProviderPresence {
    fn into(self) -> topology::provider::Node {
        topology::provider::Node {
            client_listener: self.client_listener.parse().unwrap(),
            mixnet_listener: self.mixnet_listener.parse().unwrap(),
            pub_key: self.pub_key,
            registered_clients: self
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::provider::Node> for MixProviderPresence {
    fn from(mpn: provider::Node) -> Self {
        MixProviderPresence {
            client_listener: mpn.client_listener.to_string(),
            mixnet_listener: mpn.mixnet_listener.to_string(),
            pub_key: mpn.pub_key,
            registered_clients: mpn
                .registered_clients
                .into_iter()
                .map(|c| c.into())
                .collect(),
            last_seen: mpn.last_seen,
            version: mpn.version,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixProviderClient {
    pub pub_key: String,
}

impl Into<topology::provider::Client> for MixProviderClient {
    fn into(self) -> topology::provider::Client {
        topology::provider::Client {
            pub_key: self.pub_key,
        }
    }
}

impl From<topology::provider::Client> for MixProviderClient {
    fn from(mpc: topology::provider::Client) -> Self {
        MixProviderClient {
            pub_key: mpc.pub_key,
        }
    }
}

impl PresenceEq for Vec<MixProviderPresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration.
        // We also don't care about order of registered_clients

        // since we're going to be getting rid of this very soon anyway, just clone registered
        // clients vector and sort it

        let self_set: HashSet<_> = self
            .iter()
            .map(|p| {
                (
                    &p.client_listener,
                    &p.mixnet_listener,
                    &p.pub_key,
                    &p.version,
                    p.registered_clients
                        .iter()
                        .cloned()
                        .map(|c| c.pub_key)
                        .collect::<Vec<_>>()
                        .sort(),
                )
            })
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|p| {
                (
                    &p.client_listener,
                    &p.mixnet_listener,
                    &p.pub_key,
                    &p.version,
                    p.registered_clients
                        .iter()
                        .cloned()
                        .map(|c| c.pub_key)
                        .collect::<Vec<_>>()
                        .sort(),
                )
            })
            .collect();

        self_set == other_set
    }
}
