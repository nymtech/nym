pub use nym_client_core::config::Config as BaseClientConfig;

pub use crate::config::persistence::IpPacketRouterPaths;
use nym_bin_common::logging::LoggingSettings;
use nym_network_defaults::mainnet;
use nym_service_providers_common::DEFAULT_SERVICE_PROVIDERS_DIR;
use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

mod persistence;

const DEFAULT_IP_PACKET_ROUTER_DIR: &str = "ip-packet-router";

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub base: BaseClientConfig,

    pub ip_packet_router: IpPacketRouter,

    pub storage_paths: IpPacketRouterPaths,

    pub logging: LoggingSettings,
}

impl Config {
    pub fn validate(&self) -> bool {
        // no other sections have explicit requirements (yet)
        self.base.validate()
    }

    #[doc(hidden)]
    pub fn set_no_poisson_process(&mut self) {
        self.base.set_no_poisson_process()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IpPacketRouter {
    /// Disable Poisson sending rate.
    pub disable_poisson_rate: bool,

    /// Specifies the url for an upstream source of the exit policy used by this node.
    pub upstream_exit_policy_url: Option<Url>,
}

impl Default for IpPacketRouter {
    fn default() -> Self {
        IpPacketRouter {
            disable_poisson_rate: true,
            #[allow(clippy::expect_used)]
            upstream_exit_policy_url: Some(
                mainnet::EXIT_POLICY_URL
                    .parse()
                    .expect("invalid default exit policy URL"),
            ),
        }
    }
}
