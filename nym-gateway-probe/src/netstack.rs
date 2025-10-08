// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use serde::Deserialize;
use std::ffi::{CStr, CString};

mod sys {
    use std::ffi::{c_char, c_void};

    unsafe extern "C" {
        pub unsafe fn wgPing(req: *const c_char) -> *const c_char;
        pub unsafe fn wgFreePtr(ptr: *mut c_void);
    }
}

use crate::NetstackArgs;

#[derive(serde::Serialize)]
pub struct NetstackRequest {
    private_key: String,
    public_key: String,
    endpoint: String,
    metadata_endpoint: String,
    v4_ping_config: PingConfig,
    v6_ping_config: PingConfig,
    download_timeout_sec: u64,
    awg_args: String,
}

#[derive(serde::Serialize)]
pub struct PingConfig {
    self_ip: String,
    dns: String,
    ping_hosts: Vec<String>,
    ping_ips: Vec<String>,
    num_ping: u8,
    send_timeout_sec: u64,
    recv_timeout_sec: u64,
}

impl PingConfig {
    pub fn from_netstack_args_v4(wg_ip4: &str, args: &NetstackArgs) -> Self {
        Self {
            self_ip: wg_ip4.to_string(),
            dns: args.netstack_v4_dns.clone(),
            ping_hosts: args.netstack_ping_hosts_v4.clone(),
            ping_ips: args.netstack_ping_ips_v4.clone(),
            num_ping: args.netstack_num_ping,
            send_timeout_sec: args.netstack_send_timeout_sec,
            recv_timeout_sec: args.netstack_recv_timeout_sec,
        }
    }

    pub fn from_netstack_args_v6(wg_ip6: &str, args: &NetstackArgs) -> Self {
        Self {
            self_ip: wg_ip6.to_string(),
            dns: args.netstack_v6_dns.clone(),
            ping_hosts: args.netstack_ping_hosts_v6.clone(),
            ping_ips: args.netstack_ping_ips_v6.clone(),
            num_ping: args.netstack_num_ping,
            send_timeout_sec: args.netstack_send_timeout_sec,
            recv_timeout_sec: args.netstack_recv_timeout_sec,
        }
    }
}

impl NetstackRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        wg_ip4: &str,
        wg_ip6: &str,
        private_key: &str,
        public_key: &str,
        endpoint: &str,
        metadata_endpoint: &str,
        download_timeout_sec: u64,
        awg_args: &str,
        netstack_args: NetstackArgs,
    ) -> Self {
        Self {
            private_key: private_key.to_string(),
            public_key: public_key.to_string(),
            endpoint: endpoint.to_string(),
            metadata_endpoint: metadata_endpoint.to_string(),
            awg_args: awg_args.to_string(),
            v4_ping_config: PingConfig::from_netstack_args_v4(wg_ip4, &netstack_args),
            v6_ping_config: PingConfig::from_netstack_args_v6(wg_ip6, &netstack_args),
            download_timeout_sec,
        }
    }

    #[allow(dead_code)]
    pub fn set_v4_config(&mut self, config: PingConfig) {
        self.v4_ping_config = config;
    }

    #[allow(dead_code)]
    pub fn set_v6_config(&mut self, config: PingConfig) {
        self.v6_ping_config = config;
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct NetstackRequestGo {
    pub wg_ip: String,
    private_key: String,
    public_key: String,
    endpoint: String,
    metadata_endpoint: String,
    pub dns: String,
    ip_version: u8,
    ping_hosts: Vec<String>,
    ping_ips: Vec<String>,
    num_ping: u8,
    send_timeout_sec: u64,
    recv_timeout_sec: u64,
    download_timeout_sec: u64,
    awg_args: String,
}

impl NetstackRequestGo {
    pub fn from_rust_v4(req: &NetstackRequest) -> Self {
        NetstackRequestGo {
            wg_ip: req.v4_ping_config.self_ip.clone(),
            private_key: req.private_key.clone(),
            public_key: req.public_key.clone(),
            endpoint: req.endpoint.clone(),
            metadata_endpoint: req.metadata_endpoint.clone(),
            dns: req.v4_ping_config.dns.clone(),
            ip_version: 4,
            ping_hosts: req.v4_ping_config.ping_hosts.clone(),
            ping_ips: req.v4_ping_config.ping_ips.clone(),
            num_ping: req.v4_ping_config.num_ping,
            send_timeout_sec: req.v4_ping_config.send_timeout_sec,
            recv_timeout_sec: req.v4_ping_config.recv_timeout_sec,
            download_timeout_sec: req.download_timeout_sec,
            awg_args: req.awg_args.clone(),
        }
    }

    pub fn from_rust_v6(req: &NetstackRequest) -> Self {
        NetstackRequestGo {
            wg_ip: req.v6_ping_config.self_ip.clone(),
            private_key: req.private_key.clone(),
            public_key: req.public_key.clone(),
            endpoint: req.endpoint.clone(),
            metadata_endpoint: req.metadata_endpoint.clone(),
            dns: req.v6_ping_config.dns.clone(),
            ip_version: 6,
            ping_hosts: req.v6_ping_config.ping_hosts.clone(),
            ping_ips: req.v6_ping_config.ping_ips.clone(),
            num_ping: req.v6_ping_config.num_ping,
            send_timeout_sec: req.v6_ping_config.send_timeout_sec,
            recv_timeout_sec: req.v6_ping_config.recv_timeout_sec,
            download_timeout_sec: req.download_timeout_sec,
            awg_args: req.awg_args.clone(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct NetstackResponse {
    pub can_handshake: bool,
    pub can_query_metadata: bool,
    pub sent_ips: u16,
    pub received_ips: u16,
    pub sent_hosts: u16,
    pub received_hosts: u16,
    pub can_resolve_dns: bool,
    pub downloaded_file: String,
    pub download_duration_sec: u64,
    pub downloaded_file_size_bytes: u64,
    pub download_duration_milliseconds: u64,
    pub download_error: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetstackResult {
    Response(NetstackResponse),
    Error { error: String },
}

pub fn ping(req: &NetstackRequestGo) -> anyhow::Result<NetstackResult> {
    let req_json = serde_json::to_string_pretty(req)?;
    let req_json_cstr = CString::new(req_json)?;

    // SAFETY: safety guarantees are upheld by CGO
    let response_str_ptr = unsafe { sys::wgPing(req_json_cstr.as_ptr()) };
    if response_str_ptr.is_null() {
        return Err(anyhow::anyhow!("wgPing() returned null"));
    }

    // SAFETY: safety guarantees are upheld by CGO
    let response_cstr = unsafe { CStr::from_ptr(response_str_ptr) };
    let result = match response_cstr.to_str() {
        Ok(response_str) => {
            let mut de = serde_json::Deserializer::from_str(response_str);
            let response = NetstackResult::deserialize(&mut de);

            response.context("Failed to deserialize ffi response")
        }
        Err(err) => Err(anyhow::anyhow!(
            "Failed to convert ffi response to utf8 string: {err}"
        )),
    };

    // SAFETY: freeing the pointer returned by CGO
    unsafe { sys::wgFreePtr(response_str_ptr as _) };

    result
}
