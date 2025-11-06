use crate::dns::ResolveError;

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tracing::*;

#[derive(Debug, Default, Clone)]
pub struct StaticResolver {
    static_addr_map: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
}

impl StaticResolver {
    pub fn new(static_entries: HashMap<String, Vec<IpAddr>>) -> StaticResolver {
        debug!("building static resolver");
        Self {
            static_addr_map: Arc::new(Mutex::new(static_entries)),
        }
    }

    pub fn get_addrs(&self) -> HashMap<String, Vec<IpAddr>> {
        self.static_addr_map.lock().unwrap().clone()
    }
}

impl Resolve for StaticResolver {
    fn resolve(&self, name: Name) -> Resolving {
        debug!("looking up {name:?} in static resolver");
        let addr_map = self.static_addr_map.clone();
        Box::pin(async move {
            let addr_map = addr_map.lock().unwrap();
            let lookup = match addr_map.get(name.as_str()) {
                None => return Err(ResolveError::StaticLookupMiss.into()),
                Some(addrs) => addrs,
            };
            let addrs: Addrs = Box::new(
                lookup
                    .clone()
                    .into_iter()
                    .map(|ip_addr| SocketAddr::new(ip_addr, 0)),
            );

            Ok(addrs)
        })
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;
    use std::error::Error as StdError;
    use std::str::FromStr;

    #[tokio::test]
    async fn lookup_using_static_resolver() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let example_domain = String::from("static.nymvpn.com");

        // lookup for domain for which there is no entry
        let resolver = StaticResolver::new(HashMap::new());

        let url = reqwest::dns::Name::from_str(&example_domain).unwrap();
        let result = resolver.resolve(url).await;
        assert!(result.is_err());
        match result {
            Ok(_) => panic!("lookup with empty map should fail"),
            Err(e) => assert_eq!(e.to_string(), ResolveError::StaticLookupMiss.to_string()),
        }

        // Successful lookup
        let mut addr_map = HashMap::new();
        let example_ip4: IpAddr = "10.10.10.10".parse().unwrap();
        let example_ip6: IpAddr = "dead::beef".parse().unwrap();
        addr_map.insert(example_domain.clone(), vec![example_ip4, example_ip6]);

        let url = reqwest::dns::Name::from_str(&example_domain).unwrap();
        let resolver = StaticResolver::new(addr_map);
        let mut addrs = resolver.resolve(url).await?;
        assert!(addrs.contains(&SocketAddr::new(example_ip4, 0)));
        assert!(addrs.contains(&SocketAddr::new(example_ip6, 0)));

        Ok(())
    }
}
