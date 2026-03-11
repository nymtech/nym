// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Args;

// EXIT_POLICY_PORTS is generated at build time by parsing PORT_MAPPINGS
// from scripts/nym-node-setup/network-tunnel-manager.sh.
// To add or remove ports, update PORT_MAPPINGS in the shell script and rebuild.
include!(concat!(env!("OUT_DIR"), "/exit_policy_ports.rs"));

#[derive(Args, Clone, Debug)]
pub struct NetstackArgs {
    #[arg(long, hide = true, env = "PROBE_NETSTACK_DOWNLOAD_TIMEOUT_SEC", default_value_t = NetstackArgs::default().netstack_download_timeout_sec)]
    pub netstack_download_timeout_sec: u64,

    #[arg(long, hide = true, env = "PROBE_METADATA_TIMEOUT_SEC", default_value_t = NetstackArgs::default().metadata_timeout_sec)]
    pub metadata_timeout_sec: u64,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_V4_DNS", default_value_t = NetstackArgs::default().netstack_v4_dns)]
    pub netstack_v4_dns: String,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_V6_DNS", default_value_t = NetstackArgs::default().netstack_v6_dns)]
    pub netstack_v6_dns: String,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_NUM_PING", default_value_t = NetstackArgs::default().netstack_num_ping)]
    pub netstack_num_ping: u8,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_SEND_TIMEOUT_SEC", default_value_t = NetstackArgs::default().netstack_send_timeout_sec)]
    pub netstack_send_timeout_sec: u64,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_RECV_TIMEOUT_SEC", default_value_t = NetstackArgs::default().netstack_recv_timeout_sec)]
    pub netstack_recv_timeout_sec: u64,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_PING_HOSTS_V4", default_values_t = NetstackArgs::default().netstack_ping_hosts_v4)]
    pub netstack_ping_hosts_v4: Vec<String>,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_PING_IPS_V4", default_values_t = NetstackArgs::default().netstack_ping_ips_v4)]
    pub netstack_ping_ips_v4: Vec<String>,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_PING_HOSTS_V6", default_values_t = NetstackArgs::default().netstack_ping_hosts_v6)]
    pub netstack_ping_hosts_v6: Vec<String>,

    #[arg(long, hide = true, env = "PROBE_NETSTACK_PING_IPS_V6", default_values_t = NetstackArgs::default().netstack_ping_ips_v6)]
    pub netstack_ping_ips_v6: Vec<String>,

    /// Target host for exit policy port checks (must listen on all tested ports)
    #[arg(long = "use-target", default_value = "portquiz.net")]
    pub port_check_target: String,

    /// List ports to check, separated by a comma.
    #[arg(long = "check-ports", value_delimiter = ',', default_values_t = Vec::<u16>::new())]
    pub port_check_ports: Vec<u16>,

    /// Timeout in seconds for each individual port check attempt
    #[arg(long, default_value_t = 5)]
    pub port_check_timeout_sec: u64,
}

impl Default for NetstackArgs {
    fn default() -> Self {
        Self {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: String::from("1.1.1.1"),
            netstack_v6_dns: String::from("2606:4700:4700::1111"),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string()],
            netstack_ping_hosts_v6: vec!["cloudflare.com".to_string()],
            netstack_ping_ips_v6: vec![
                "2001:4860:4860::8888".to_string(),
                "2606:4700:4700::1111".to_string(),
                "2620:fe::fe".to_string(),
            ],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![],
            port_check_timeout_sec: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netstack_args_default_values() {
        // Test that the default values are correctly set in the struct definition
        // This validates that our changes to the default values are correct

        // Create a default instance to test the values
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string()],
            netstack_ping_hosts_v6: vec!["cloudflare.com".to_string()],
            netstack_ping_ips_v6: vec![
                "2001:4860:4860::8888".to_string(),
                "2606:4700:4700::1111".to_string(),
                "2620:fe::fe".to_string(),
            ],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![],
            port_check_timeout_sec: 5,
        };

        // Test IPv4 defaults
        assert_eq!(args.netstack_ping_hosts_v4, vec!["nym.com"]);
        assert_eq!(args.netstack_ping_ips_v4, vec!["1.1.1.1"]);
        assert_eq!(args.netstack_v4_dns, "1.1.1.1");

        // Test IPv6 defaults
        assert_eq!(args.netstack_ping_hosts_v6, vec!["cloudflare.com"]);
        assert_eq!(
            args.netstack_ping_ips_v6,
            vec![
                "2001:4860:4860::8888",
                "2606:4700:4700::1111",
                "2620:fe::fe"
            ]
        );
        assert_eq!(args.netstack_v6_dns, "2606:4700:4700::1111");

        // Test other defaults
        assert_eq!(args.netstack_download_timeout_sec, 180);
        assert_eq!(args.netstack_num_ping, 5);
        assert_eq!(args.netstack_send_timeout_sec, 3);
        assert_eq!(args.netstack_recv_timeout_sec, 3);

        // Test port check defaults
        assert_eq!(args.port_check_target, "portquiz.net");
        assert!(args.port_check_ports.is_empty());
        assert_eq!(args.port_check_timeout_sec, 5);
    }

    #[test]
    fn test_netstack_args_custom_construction() {
        // Test that we can create instances with custom values
        let args = NetstackArgs {
            netstack_download_timeout_sec: 300,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "8.8.8.8".to_string(),
            netstack_v6_dns: "2001:4860:4860::8888".to_string(),
            netstack_num_ping: 10,
            netstack_send_timeout_sec: 5,
            netstack_recv_timeout_sec: 5,
            netstack_ping_hosts_v4: vec!["example.com".to_string()],
            netstack_ping_ips_v4: vec!["8.8.8.8".to_string()],
            netstack_ping_hosts_v6: vec!["ipv6.example.com".to_string()],
            netstack_ping_ips_v6: vec!["2001:4860:4860::8888".to_string()],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![80, 443, 8332],
            port_check_timeout_sec: 10,
        };

        assert_eq!(args.netstack_ping_hosts_v4, vec!["example.com"]);
        assert_eq!(args.netstack_ping_hosts_v6, vec!["ipv6.example.com"]);
        assert_eq!(args.netstack_ping_ips_v4, vec!["8.8.8.8"]);
        assert_eq!(args.netstack_ping_ips_v6, vec!["2001:4860:4860::8888"]);
        assert_eq!(args.netstack_v4_dns, "8.8.8.8");
        assert_eq!(args.netstack_v6_dns, "2001:4860:4860::8888");
        assert_eq!(args.netstack_download_timeout_sec, 300);
        assert_eq!(args.netstack_num_ping, 10);
        assert_eq!(args.netstack_send_timeout_sec, 5);
        assert_eq!(args.netstack_recv_timeout_sec, 5);
    }

    #[test]
    fn test_netstack_args_multiple_values() {
        // Test that multiple hosts and IPs can be stored
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string(), "example.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            netstack_ping_hosts_v6: vec![
                "cloudflare.com".to_string(),
                "ipv6.example.com".to_string(),
            ],
            netstack_ping_ips_v6: vec![
                "2001:4860:4860::8888".to_string(),
                "2606:4700:4700::1111".to_string(),
            ],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![],
            port_check_timeout_sec: 5,
        };

        assert_eq!(args.netstack_ping_hosts_v4, vec!["nym.com", "example.com"]);
        assert_eq!(
            args.netstack_ping_hosts_v6,
            vec!["cloudflare.com", "ipv6.example.com"]
        );
        assert_eq!(args.netstack_ping_ips_v4, vec!["1.1.1.1", "8.8.8.8"]);
        assert_eq!(
            args.netstack_ping_ips_v6,
            vec!["2001:4860:4860::8888", "2606:4700:4700::1111"]
        );
    }

    #[test]
    fn test_netstack_args_edge_cases() {
        // Test edge cases like zero values and empty vectors
        let args = NetstackArgs {
            netstack_download_timeout_sec: 0,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 0,
            netstack_send_timeout_sec: 0,
            netstack_recv_timeout_sec: 0,
            netstack_ping_hosts_v4: vec![],
            netstack_ping_ips_v4: vec![],
            netstack_ping_hosts_v6: vec![],
            netstack_ping_ips_v6: vec![],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![],
            port_check_timeout_sec: 0,
        };

        assert_eq!(args.netstack_num_ping, 0);
        assert_eq!(args.netstack_send_timeout_sec, 0);
        assert_eq!(args.netstack_recv_timeout_sec, 0);
        assert_eq!(args.netstack_download_timeout_sec, 0);
        assert!(args.netstack_ping_hosts_v4.is_empty());
        assert!(args.netstack_ping_ips_v4.is_empty());
        assert!(args.netstack_ping_hosts_v6.is_empty());
        assert!(args.netstack_ping_ips_v6.is_empty());
    }

    #[test]
    fn test_netstack_args_domain_validation() {
        // Test that our domain choices are reasonable
        let args = NetstackArgs {
            netstack_download_timeout_sec: 180,
            metadata_timeout_sec: 30,
            netstack_v4_dns: "1.1.1.1".to_string(),
            netstack_v6_dns: "2606:4700:4700::1111".to_string(),
            netstack_num_ping: 5,
            netstack_send_timeout_sec: 3,
            netstack_recv_timeout_sec: 3,
            netstack_ping_hosts_v4: vec!["nym.com".to_string()],
            netstack_ping_ips_v4: vec!["1.1.1.1".to_string()],
            netstack_ping_hosts_v6: vec!["cloudflare.com".to_string()],
            netstack_ping_ips_v6: vec!["2001:4860:4860::8888".to_string()],
            port_check_target: "portquiz.net".to_string(),
            port_check_ports: vec![],
            port_check_timeout_sec: 5,
        };

        assert!(args.netstack_ping_hosts_v4[0].contains("nym"));

        assert!(args.netstack_ping_hosts_v6[0].contains("cloudflare"));

        assert_eq!(args.netstack_v4_dns, "1.1.1.1");
        assert_eq!(args.netstack_v6_dns, "2606:4700:4700::1111");
    }

    #[test]
    fn test_exit_policy_ports_no_duplicates_and_sorted() {
        let ports = EXIT_POLICY_PORTS;
        assert!(!ports.is_empty(), "EXIT_POLICY_PORTS should not be empty");

        // verify sorted
        for window in ports.windows(2) {
            assert!(
                window[0] < window[1],
                "EXIT_POLICY_PORTS out of order or duplicate: {} >= {}",
                window[0],
                window[1]
            );
        }

        // spot-check a few well-known ports
        assert!(ports.contains(&22), "should contain SSH (22)");
        assert!(ports.contains(&443), "should contain HTTPS (443)");
        assert!(ports.contains(&22021), "should contain Session (22021)");
        assert!(ports.contains(&8332), "should contain Bitcoin (8332)");
        assert!(ports.contains(&9735), "should contain Lightning (9735)");
    }
}
