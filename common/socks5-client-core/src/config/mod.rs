// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use nym_client_core::config::Config as BaseClientConfig;
use nym_config::defaults::DEFAULT_SOCKS5_LISTENING_PORT;
use nym_config::OptionalSet;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

pub mod old_config_v1_1_20_2;
pub mod old_config_v1_1_30;
pub mod old_config_v1_1_33;

pub use nym_service_providers_common::interface::ProviderInterfaceVersion;
pub use nym_socks5_requests::Socks5ProtocolVersion;

const DEFAULT_CONNECTION_START_SURBS: u32 = 20;
const DEFAULT_PER_REQUEST_SURBS: u32 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    pub socks5: Socks5,
}

impl Config {
    pub fn new<S: Into<String>>(id: S, version: S, provider_mix_address: S) -> Self {
        Config {
            base: BaseClientConfig::new(id, version),
            socks5: Socks5::new(provider_mix_address),
        }
    }

    pub fn from_base(base: BaseClientConfig, socks5: Socks5) -> Self {
        Config { base, socks5 }
    }

    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.socks5.bind_address = SocketAddr::new(self.socks5.bind_address.ip(), port);
        self
    }

    #[must_use]
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.socks5.bind_address = SocketAddr::new(ip, self.socks5.bind_address.port());
        self
    }

    pub fn with_anonymous_replies(mut self, anonymous_replies: bool) -> Self {
        self.socks5.send_anonymously = anonymous_replies;
        self
    }

    // poor man's 'builder' method
    pub fn with_base<F, T>(mut self, f: F, val: T) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.base = f(self.base, val);
        self
    }

    // helper methods to use `OptionalSet` trait. Those are defined due to very... ehm. 'specific' structure of this config
    // (plz, lets refactor it)
    pub fn with_optional_base<F, T>(mut self, f: F, val: Option<T>) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
    {
        self.base = self.base.with_optional(f, val);
        self
    }

    pub fn with_optional_base_env<F, T>(mut self, f: F, val: Option<T>, env_var: &str) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        T: FromStr,
        <T as FromStr>::Err: Debug,
    {
        self.base = self.base.with_optional_env(f, val, env_var);
        self
    }

    pub fn with_optional_base_custom_env<F, T, G>(
        mut self,
        f: F,
        val: Option<T>,
        env_var: &str,
        parser: G,
    ) -> Self
    where
        F: Fn(BaseClientConfig, T) -> BaseClientConfig,
        G: Fn(&str) -> T,
    {
        self.base = self.base.with_optional_custom_env(f, val, env_var, parser);
        self
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5 {
    /// The address on which the client will be listening for incoming requests
    /// (default: 127.0.0.1:1080)
    pub bind_address: SocketAddr,

    /// The mix address of the provider to which all requests are going to be sent.
    pub provider_mix_address: String,

    /// The version of the 'service provider' this client is going to use in its communication with the
    /// specified socks5 provider.
    // if in doubt, use the legacy version as initially nobody will be using the updated binaries
    #[serde(default)]
    pub provider_interface_version: ProviderInterfaceVersion,

    #[serde(default)]
    pub socks5_protocol_version: Socks5ProtocolVersion,

    /// Specifies whether this client is going to use an anonymous sender tag for communication with the service provider.
    /// While this is going to hide its actual address information, it will make the actual communication
    /// slower and consume nearly double the bandwidth as it will require sending reply SURBs.
    ///
    /// Note that some service providers might not support this.
    #[serde(default)]
    pub send_anonymously: bool,

    #[serde(default)]
    pub socks5_debug: Socks5Debug,
}

impl Socks5 {
    pub fn new<S: Into<String>>(provider_mix_address: S) -> Self {
        Socks5 {
            bind_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                DEFAULT_SOCKS5_LISTENING_PORT,
            ),
            provider_mix_address: provider_mix_address.into(),
            provider_interface_version: ProviderInterfaceVersion::Legacy,
            socks5_protocol_version: Socks5ProtocolVersion::Legacy,
            send_anonymously: false,
            socks5_debug: Default::default(),
        }
    }

    pub fn get_provider_mix_address(&self) -> Recipient {
        Recipient::try_from_base58_string(&self.provider_mix_address)
            .expect("malformed provider address")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Socks5Debug {
    /// Number of reply SURBs attached to each `Request::Connect` message.
    pub connection_start_surbs: u32,

    /// Number of reply SURBs attached to each `Request::Send` message.
    pub per_request_surbs: u32,
}

impl Default for Socks5Debug {
    fn default() -> Self {
        Socks5Debug {
            connection_start_surbs: DEFAULT_CONNECTION_START_SURBS,
            per_request_surbs: DEFAULT_PER_REQUEST_SURBS,
        }
    }
}
