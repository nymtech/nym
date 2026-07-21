use nym_ip_packet_requests::IpPair;
use nym_network_defaults::constants::mixnet_vpn::{
    NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6,
};
use std::net::Ipv6Addr;
use std::{collections::HashMap, net::Ipv4Addr};

// Find an available IP address in self.connected_clients
// TODO: make this nicer
fn generate_random_ips_within_subnet<R: rand::Rng>(rng: &mut R) -> IpPair {
    // Generate a random number in the range 2-65535
    let last_bytes: u16 = rand::Rng::gen_range(rng, 2..=65534);
    let before_last_byte = (last_bytes >> 8) as u8;
    let last_byte = (last_bytes & 255) as u8;
    let ipv4 = Ipv4Addr::new(10, 0, before_last_byte, last_byte);
    let ipv6 = Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, last_bytes);
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
    let mut rng = rand::thread_rng();
    let mut new_ips = generate_random_ips_within_subnet(&mut rng);
    let mut tries = 0;
    let tun_ips = IpPair::new(NYM_TUN_DEVICE_ADDRESS_V4, NYM_TUN_DEVICE_ADDRESS_V6);

    while is_ip_taken(
        connected_clients_ipv4,
        connected_clients_ipv6,
        tun_ips,
        new_ips,
    ) {
        new_ips = generate_random_ips_within_subnet(&mut rng);
        tries += 1;
        if tries > 100 {
            return None;
        }
    }
    Some(new_ips)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Hammer the brute-force IP allocator as the /16 pool fills, measuring
    // per-allocation latency and how often it falsely returns NoAvailableIp
    // despite free addresses. Run: cargo test -p nym-ip-packet-router
    // allocator_latency_vs_fill -- --nocapture --ignored
    #[test]
    #[ignore]
    fn allocator_latency_vs_fill() {
        use std::net::Ipv6Addr;
        use std::time::Instant;
        let total: u32 = 65533;
        println!(
            "\n{:>5} {:>7} {:>8} {:>12} {:>14}",
            "fill%", "clients", "allocs", "us/alloc", "NoAvailableIp"
        );
        for pct in [0u32, 25, 50, 75, 90, 95, 98, 99] {
            let fill = (total as u64 * pct as u64 / 100) as u16;
            let mut v4: HashMap<Ipv4Addr, ()> = HashMap::with_capacity(fill as usize);
            let mut v6: HashMap<Ipv6Addr, ()> = HashMap::with_capacity(fill as usize);
            for lb in 2u16..2u16.saturating_add(fill) {
                v4.insert(Ipv4Addr::new(10, 0, (lb >> 8) as u8, (lb & 255) as u8), ());
                v6.insert(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, lb), ());
            }
            let n = 5000;
            let start = Instant::now();
            let mut none = 0;
            for _ in 0..n {
                if find_new_ips(&v4, &v6).is_none() {
                    none += 1;
                }
            }
            let el = start.elapsed();
            println!(
                "{:>5} {:>7} {:>8} {:>12.2} {:>10} ({:>4.1}%)",
                pct,
                v4.len(),
                n,
                el.as_nanos() as f64 / n as f64 / 1000.0,
                none,
                100.0 * none as f64 / n as f64
            );
        }
    }

    #[test]
    fn verify_ip_generation() {
        let mut map = HashSet::with_capacity(65533);
        let mut rng = rand::rngs::mock::StepRng::new(0, 65540);
        for _ in 2..65535 {
            let pair = generate_random_ips_within_subnet(&mut rng);
            assert!(!map.contains(&pair));
            map.insert(pair);
        }
        let pair = generate_random_ips_within_subnet(&mut rng);
        assert!(map.contains(&pair));
    }
}
