use nym_client_core::config::{Client as ClientConfig, DebugConfig};
use nym_network_defaults::NymNetworkDetails;
use nym_socks5_client_core::config::BaseClientConfig;
use url::Url;

const DEFAULT_SDK_CLIENT_ID: &str = "_default-nym-sdk-client";

/// Config struct for [`crate::mixnet::MixnetClient`]
#[derive(Default)]
pub struct Config {
    /// If the user has explicitly specified a gateway.
    pub user_chosen_gateway: Option<String>,

    /// The details of the network we're using. It defaults to the mainnet network.
    pub network_details: NymNetworkDetails,

    /// Whether to attempt to use gateway with bandwidth credential requirement.
    pub enabled_credentials_mode: bool,

    /// Flags controlling all sorts of internal client behaviour.
    /// Changing these risk compromising network anonymity!
    pub debug_config: DebugConfig,
}

impl Config {
    // I really dislike this workaround.
    pub fn as_base_client_config(
        &self,
        nyxd_endpoints: Vec<Url>,
        nym_api_endpoints: Vec<Url>,
    ) -> BaseClientConfig {
        BaseClientConfig::from_client_config(
            ClientConfig::new(
                DEFAULT_SDK_CLIENT_ID,
                env!("CARGO_PKG_VERSION"),
                !self.enabled_credentials_mode,
                nyxd_endpoints,
                nym_api_endpoints,
            ),
            self.debug_config,
        )
    }
}
