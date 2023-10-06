#[derive(Default)]
pub(crate) struct NetworkTable<T> {
    pub ips: ip_network_table::IpNetworkTable<T>,
}

impl<T> NetworkTable<T> {
    pub(crate) fn new() -> Self {
        Self {
            ips: ip_network_table::IpNetworkTable::new(),
        }
    }
}
