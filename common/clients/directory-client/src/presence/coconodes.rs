use super::PresenceEq;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use topology::coco;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::coco::Node> for CocoPresence {
    fn into(self) -> topology::coco::Node {
        topology::coco::Node {
            host: self.host,
            pub_key: self.pub_key,
            last_seen: self.last_seen,
            version: self.version,
        }
    }
}

impl From<topology::coco::Node> for CocoPresence {
    fn from(cn: coco::Node) -> Self {
        CocoPresence {
            host: cn.host,
            pub_key: cn.pub_key,
            last_seen: cn.last_seen,
            version: cn.version,
        }
    }
}

impl PresenceEq for Vec<CocoPresence> {
    fn presence_eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // we can't take the whole thing into set as it does not implement 'Eq' and we can't
        // derive it as we don't want to take 'last_seen' into consideration
        let self_set: HashSet<_> = self
            .iter()
            .map(|c| (&c.host, &c.pub_key, &c.version))
            .collect();
        let other_set: HashSet<_> = other
            .iter()
            .map(|c| (&c.host, &c.pub_key, &c.version))
            .collect();

        self_set == other_set
    }
}
