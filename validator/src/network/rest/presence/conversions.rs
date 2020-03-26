use crate::network::rest::presence::models::Mixnode as PresenceMixnode;
use crate::services::mixmining::Mixnode as ServiceMixnode;
use std::convert::From;
use std::time::{SystemTime, UNIX_EPOCH};

impl From<PresenceMixnode> for ServiceMixnode {
    fn from(value: PresenceMixnode) -> ServiceMixnode {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        ServiceMixnode {
            host: value.host,
            last_seen: timestamp,
            location: value.location,
            public_key: value.public_key,
            stake: 0,
            version: value.version,
        }
    }
}

#[cfg(test)]
mod test_presence_conversions_for_mixmining_service {
    use super::*;

    #[test]
    fn test_converting_presence_mixnode_to_mixmining_service_mixnode() {
        let presence_mixnode = PresenceMixnode {
            host: "foo.org".to_owned(),
            public_key: "abc".to_owned(),
            location: "London".to_owned(),
            version: "1.0.0".to_owned(),
        };

        let result: ServiceMixnode = presence_mixnode.clone().into();
        assert_eq!(result.host, presence_mixnode.host);
        assert_eq!(result.public_key, presence_mixnode.public_key);
        assert_eq!(result.location, presence_mixnode.location);
        assert_eq!(result.stake, 0);
        assert_eq!(result.version, presence_mixnode.version);
        // I'm not going to test the last_seen timestamp as I can't be bothered
        // setting up a fake clock right now.
        // The behaviour is: it should set time to SystemTime::now().
    }
}
