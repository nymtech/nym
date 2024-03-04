use nym_ip_packet_requests::IpPair;
use std::net::Ipv6Addr;
use std::{collections::HashMap, net::Ipv4Addr};

use crate::constants::{TUN_DEVICE_ADDRESS_V4, TUN_DEVICE_ADDRESS_V6};

// Find an available IP address in self.connected_clients
// TODO: make this nicer
fn generate_random_ips_within_subnet() -> IpPair {
    let mut rng = rand::thread_rng();
    // Generate a random number in the range 1-254
    let last_octet = rand::Rng::gen_range(&mut rng, 1..=254);
    let ipv4 = Ipv4Addr::new(10, 0, 0, last_octet);
    let ipv6 = Ipv6Addr::new(0x2001, 0x0db8, 0xa160, 0, 0, 0, 0, last_octet as u16);
    IpPair::new(ipv4, ipv6)
}

fn is_ip_taken<T>(
    connected_clients_ipv4: &HashMap<Ipv4Addr, T>,
    connected_clients_ipv6: &HashMap<Ipv6Addr, T>,
    tun_ips: IpPair,
    ips: IpPair,
) -> bool {
    connected_clients_ipv4.contains_key(&ips.ipv4)
        || connected_clients_ipv6.contains_key(&ips.ipv6)
        || ips.ipv4 == tun_ips.ipv4
        || ips.ipv6 == tun_ips.ipv6
}

// TODO: brute force approach. We could consider using a more efficient algorithm
pub(crate) fn find_new_ips<T>(
    connected_clients_ipv4: &HashMap<Ipv4Addr, T>,
    connected_clients_ipv6: &HashMap<Ipv6Addr, T>,
) -> Option<IpPair> {
    let mut new_ips = generate_random_ips_within_subnet();
    let mut tries = 0;
    let tun_ips = IpPair::new(TUN_DEVICE_ADDRESS_V4, TUN_DEVICE_ADDRESS_V6);

    while is_ip_taken(
        connected_clients_ipv4,
        connected_clients_ipv6,
        tun_ips,
        new_ips,
    ) {
        new_ips = generate_random_ips_within_subnet();
        tries += 1;
        if tries > 100 {
            return None;
        }
    }
    Some(new_ips)
}
