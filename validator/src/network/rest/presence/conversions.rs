use crate::network::rest::presence::models::Mixnode as RestMixnode;
use crate::network::rest::presence::models::Topology as RestTopology;
use crate::services::mixmining::models::Mixnode as ServiceMixnode;
use crate::services::mixmining::models::Topology as ServiceTopology;
use std::convert::From;
use std::time::{SystemTime, UNIX_EPOCH};

impl From<RestMixnode> for ServiceMixnode {
    fn from(value: RestMixnode) -> ServiceMixnode {
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

impl From<RestTopology> for ServiceTopology {
    fn from(value: RestTopology) -> ServiceTopology {
        let mut converted_mixnodes: Vec<ServiceMixnode> = Vec::new();
        for mixnode in value.mixnodes {
            converted_mixnodes.push(mixnode.into());
        }
        ServiceTopology {
            mixnodes: converted_mixnodes.to_vec(),
            service_providers: vec![], // add these when they exist
            validators: vec![],        // add these when they exist
        }
    }
}

#[cfg(test)]
mod test_presence_conversions_for_mixmining_service {

    fn mixnode_fixture() -> RestMixnode {
        RestMixnode {
            host: "foo.org".to_owned(),
            public_key: "abc".to_owned(),
            location: "London".to_owned(),
            version: "1.0.0".to_owned(),
        }
    }

    use super::*;

    #[test]
    fn test_converting_rest_mixnode_to_mixmining_service_mixnode() {
        let rest_mixnode = mixnode_fixture();
        let result: ServiceMixnode = rest_mixnode.clone().into();
        assert_eq!(result.host, rest_mixnode.host);
        assert_eq!(result.public_key, rest_mixnode.public_key);
        assert_eq!(result.location, rest_mixnode.location);
        assert_eq!(result.stake, 0);
        assert_eq!(result.version, rest_mixnode.version);
        // I'm not going to test the last_seen timestamp as I can't be bothered
        // setting up a fake clock right now.
        // The behaviour is: it should set time to SystemTime::now().
    }

    #[test]
    fn test_building_service_mixnode_from_rest_mixnode() {
        let rest_mixnode = RestMixnode {
            host: "foo.org".to_owned(),
            // last_seen: 1234,
            location: "London".to_owned(),
            public_key: "abc".to_owned(),
            // stake: 0,
            version: "1.0.0".to_owned(),
        };
        let result = ServiceMixnode::from(rest_mixnode.clone());
        assert_eq!(result.host, rest_mixnode.host);
        assert_eq!(result.public_key, rest_mixnode.public_key);
        assert_eq!(result.location, rest_mixnode.location);
        assert_eq!(result.stake, 0);
        assert_eq!(result.version, rest_mixnode.version);
        // I'm not going to test the last_seen timestamp as I can't be bothered
        // setting up a fake clock right now.
        // The behaviour is: it should set time to SystemTime::now().
    }

    #[test]
    fn test_converting_service_topology_into_rest_topology() {
        let rest_topology = RestTopology {
            mixnodes: vec![mixnode_fixture()],
            service_providers: vec![],
            validators: vec![],
        };

        let service_topology: ServiceTopology = rest_topology.into();
        let service_mixnode: ServiceMixnode = mixnode_fixture().into();
        assert_eq!(service_mixnode, service_topology.mixnodes[0]);
    }
}
