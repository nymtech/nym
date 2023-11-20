use std::net::IpAddr;

use ip_network::IpNetwork;
use ip_network_table::IpNetworkTable;

#[derive(Default)]
pub struct NetworkTable<T> {
    ips: IpNetworkTable<T>,
}

impl<T> NetworkTable<T> {
    pub fn new() -> Self {
        Self {
            ips: IpNetworkTable::new(),
        }
    }

    pub fn insert<N: Into<IpNetwork>>(&mut self, network: N, data: T) -> Option<T> {
        self.ips.insert(network, data)
    }

    pub fn longest_match<I: Into<IpAddr>>(&self, ip: I) -> Option<(IpNetwork, &T)> {
        self.ips.longest_match(ip)
    }
}
