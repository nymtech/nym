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

impl From<ServiceMixnode> for RestMixnode {
    fn from(value: ServiceMixnode) -> RestMixnode {
        RestMixnode {
            host: value.host,
            location: value.location,
            public_key: value.public_key,
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
            service_providers: vec![], // add these when conversions exist
            validators: vec![],        // add these when conversions exist
        }
    }
}

impl From<ServiceTopology> for RestTopology {
    fn from(value: ServiceTopology) -> RestTopology {
        let mut converted_mixnodes: Vec<RestMixnode> = Vec::new();
        for mixnode in value.mixnodes {
            converted_mixnodes.push(mixnode.into());
        }
        RestTopology {
            mixnodes: converted_mixnodes.to_vec(),
            service_providers: vec![], // add these when conversions exist
            validators: vec![],        // add these when conversions exist
        }
    }
}

#[cfg(test)]
mod test_presence_conversions_for_mixmining_service {
    fn rest_mixnode_fixture() -> RestMixnode {
        RestMixnode {
            host: "foo.org".to_owned(),
            public_key: "abc".to_owned(),
            location: "London".to_owned(),
            version: "1.0.0".to_owned(),
        }
    }

    fn service_mixnode_fixture() -> ServiceMixnode {
        ServiceMixnode {
            host: "foo.org".to_owned(),
            public_key: "abc".to_owned(),
            last_seen: 1234,
            location: "London".to_owned(),
            stake: 0,
            version: "1.0.0".to_owned(),
        }
    }

    use super::*;

    #[test]
    fn test_building_service_mixnode_from_rest_mixnode() {
        let rest_mixnode = rest_mixnode_fixture();
        let service_mixnode = ServiceMixnode::from(rest_mixnode.clone());
        assert_eq!(service_mixnode.host, rest_mixnode.host);
        assert_eq!(service_mixnode.public_key, rest_mixnode.public_key);
        assert_eq!(service_mixnode.location, rest_mixnode.location);
        assert_eq!(service_mixnode.stake, 0);
        assert_eq!(service_mixnode.version, rest_mixnode.version);
        // I'm not going to test the last_seen timestamp as I can't be bothered
        // setting up a fake clock right now.
        // The behaviour is: it should set time to SystemTime::now().
    }

    #[test]
    fn test_building_rest_mixnode_from_service_mixnode() {
        let service_mixnode = service_mixnode_fixture();
        let rest_mixnode = RestMixnode::from(service_mixnode.clone());
        assert_eq!(rest_mixnode.host, service_mixnode.host);
        assert_eq!(rest_mixnode.public_key, service_mixnode.public_key);
        assert_eq!(rest_mixnode.location, service_mixnode.location);
        assert_eq!(rest_mixnode.version, service_mixnode.version);
    }

    #[test]
    fn test_building_service_topology_from_rest_topology() {
        let rest_mixnode = rest_mixnode_fixture();
        let rest_topology = RestTopology {
            mixnodes: vec![rest_mixnode.clone()],
            service_providers: vec![],
            validators: vec![],
        };

        let service_topology = ServiceTopology::from(rest_topology);
        let service_mixnode = ServiceMixnode::from(rest_mixnode);
        assert_eq!(service_mixnode, service_topology.mixnodes[0]);
    }
}
