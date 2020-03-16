use serde::{Deserialize, Serialize};
use topology::coco;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub location: String,
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl Into<topology::coco::Node> for CocoPresence {
    fn into(self) -> topology::coco::Node {
        topology::coco::Node {
            location: self.location,
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
            location: cn.location,
            host: cn.host,
            pub_key: cn.pub_key,
            last_seen: cn.last_seen,
            version: cn.version,
        }
    }
}
