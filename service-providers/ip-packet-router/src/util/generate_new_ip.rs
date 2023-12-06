use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

use crate::{constants::TUN_DEVICE_ADDRESS, mixnet_listener::ConnectedClient};

// Find an available IP address in self.connected_clients
// TODO: make this nicer
fn generate_random_ip_within_subnet() -> Ipv4Addr {
    let mut rng = rand::thread_rng();
    // Generate a random number in the range 1-254
    let last_octet = rand::Rng::gen_range(&mut rng, 1..=254);
    Ipv4Addr::new(10, 0, 0, last_octet)
}

fn is_ip_taken(
    connected_clients: &HashMap<IpAddr, ConnectedClient>,
    tun_ip: Ipv4Addr,
    ip: Ipv4Addr,
) -> bool {
    connected_clients.contains_key(&ip.into()) || ip == tun_ip
}

// TODO: brute force approach. We could consider using a more efficient algorithm
pub(crate) fn find_new_ip(connected_clients: &HashMap<IpAddr, ConnectedClient>) -> Option<IpAddr> {
    let mut new_ip = generate_random_ip_within_subnet();
    let mut tries = 0;
    let tun_ip = TUN_DEVICE_ADDRESS.parse::<Ipv4Addr>().unwrap();
    while is_ip_taken(connected_clients, tun_ip, new_ip) {
        new_ip = generate_random_ip_within_subnet();
        tries += 1;
        if tries > 100 {
            return None;
        }
    }
    Some(new_ip.into())
}
