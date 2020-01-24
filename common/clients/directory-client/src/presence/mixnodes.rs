use super::PresenceEq;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;
use topology::mix;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixNodePresence {
    pub host: String,
    pub pub_key: String,
    pub layer: u64,
    pub last_seen: u64,
    pub version: String,
}

impl TryInto<topology::mix::Node> for MixNodePresence {
    type Error = io::Error;

    fn try_into(self) -> Result<topology::mix::Node, Self::Error> {
        let resolved_hostname = self.host.to_socket_addrs()?.next();
        if resolved_hostname.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "no valid socket address",
            ));
        }

        Ok(topology::mix::Node {
            host: resolved_hostname.unwrap(),
            pub_key: self.pub_key,
            layer: self.layer,
            last_seen: self.last_seen,
            version: self.version,
        })
    }
}

impl From<topology::mix::Node> for MixNodePresence {
    fn from(mn: mix::Node) -> Self {
        MixNodePresence {
            host: mn.host.to_string(),
            pub_key: mn.pub_key,
            layer: mn.layer,
            last_seen: mn.last_seen,
            version: mn.version,
        }
    }
}

impl PresenceEq for Vec<MixNodePresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration
        let self_set: HashSet<_> = self
            .iter()
            .map(|m| (&m.host, &m.pub_key, &m.version, &m.layer))
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|m| (&m.host, &m.pub_key, &m.version, &m.layer))
            .collect();

        self_set == other_set
    }
}
