use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub node: String,
    pub used_entry: String,
    pub outcome: ProbeOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeOutcome {
    pub as_entry: Entry,
    pub as_exit: Option<Exit>,
    pub wg: Option<WgProbeResults>,
}

impl ProbeOutcome {
    pub fn is_fully_operational_entry(&self) -> bool {
        if let Entry::Tested(entry_test_result) = &self.as_entry {
            entry_test_result.can_connect && entry_test_result.can_route
        } else {
            false
        }
    }

    pub fn is_fully_operational_exit(&self) -> bool {
        if let Entry::Tested(entry_test_result) = &self.as_entry {
            entry_test_result.can_connect
                && entry_test_result.can_route
                && self.as_exit.as_ref().is_some_and(|exit| {
                    exit.can_connect
                        && exit.can_route_ip_v4
                        && exit.can_route_ip_external_v4
                        && exit.can_route_ip_v6
                        && exit.can_route_ip_external_v6
                })
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename = "wg")]
pub struct WgProbeResults {
    pub can_register: bool,

    pub can_handshake_v4: bool,
    pub can_resolve_dns_v4: bool,
    pub ping_hosts_performance_v4: f32,
    pub ping_ips_performance_v4: f32,

    pub can_handshake_v6: bool,
    pub can_resolve_dns_v6: bool,
    pub ping_hosts_performance_v6: f32,
    pub ping_ips_performance_v6: f32,

    pub download_duration_sec_v4: u64,
    pub downloaded_file_v4: String,
    pub download_error_v4: String,

    pub download_duration_sec_v6: u64,
    pub downloaded_file_v6: String,
    pub download_error_v6: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::enum_variant_names)]
pub enum Entry {
    Tested(EntryTestResult),
    NotTested,
    EntryFailure,
}

impl From<EntryTestResult> for Entry {
    fn from(value: EntryTestResult) -> Self {
        Entry::Tested(value)
    }
}

impl Entry {
    pub fn fail_to_connect() -> Self {
        EntryTestResult {
            can_connect: false,
            can_route: false,
        }
        .into()
    }

    pub fn fail_to_route() -> Self {
        EntryTestResult {
            can_connect: true,
            can_route: false,
        }
        .into()
    }

    pub fn success() -> Self {
        EntryTestResult {
            can_connect: true,
            can_route: true,
        }
        .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTestResult {
    pub can_connect: bool,
    pub can_route: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exit {
    pub can_connect: bool,
    pub can_route_ip_v4: bool,
    pub can_route_ip_external_v4: bool,
    pub can_route_ip_v6: bool,
    pub can_route_ip_external_v6: bool,
}

impl Exit {
    pub fn fail_to_connect() -> Self {
        Self {
            can_connect: false,
            can_route_ip_v4: false,
            can_route_ip_external_v4: false,
            can_route_ip_v6: false,
            can_route_ip_external_v6: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IpPingReplies {
    pub ipr_tun_ip_v4: bool,
    pub ipr_tun_ip_v6: bool,
    pub external_ip_v4: bool,
    pub external_ip_v6: bool,
}

impl IpPingReplies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_event(&mut self, event: &ConnectionStatusEvent) {
        match event {
            ConnectionStatusEvent::MixnetSelfPing => {}
            ConnectionStatusEvent::Icmpv4IprTunDevicePingReply => self.ipr_tun_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprTunDevicePingReply => self.ipr_tun_ip_v6 = true,
            ConnectionStatusEvent::Icmpv4IprExternalPingReply => self.external_ip_v4 = true,
            ConnectionStatusEvent::Icmpv6IprExternalPingReply => self.external_ip_v6 = true,
        }
    }
}

// Events that are reported by other tasks to the connection monitor
#[derive(Debug)]
pub enum ConnectionStatusEvent {
    MixnetSelfPing,
    Icmpv4IprTunDevicePingReply,
    Icmpv6IprTunDevicePingReply,
    Icmpv4IprExternalPingReply,
    Icmpv6IprExternalPingReply,
}
