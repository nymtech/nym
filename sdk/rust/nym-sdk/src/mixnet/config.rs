use nym_client_core::config::DebugConfig;
use nym_network_defaults::mainnet;
use url::Url;

/// Config struct for [`crate::mixnet::MixnetClient`]
pub struct Config {
    /// If the user has explicitly specified a gateway.
    pub user_chosen_gateway: Option<String>,

    /// List of nyxd endpoints
    pub nyxd_endpoints: Vec<Url>,

    /// List of nym-api endpoints
    pub nym_api_endpoints: Vec<Url>,

    /// Flags controlling all sorts of internal client behaviour.
    /// Changing these risk compromising network anonymity!
    pub debug_config: DebugConfig,
}

impl Default for Config {
    fn default() -> Self {
        let nym_api_endpoints = vec![mainnet::NYM_API.to_string().parse().unwrap()];
        let nyxd_endpoints = vec![mainnet::NYXD_URL.to_string().parse().unwrap()];
        Self {
            user_chosen_gateway: Default::default(),
            nyxd_endpoints,
            nym_api_endpoints,
            debug_config: Default::default(),
        }
    }
}

impl Config {
    /// Creates a new [`Config`].
    pub fn new(
        user_chosen_gateway: Option<String>,
        nyxd_endpoints: Vec<Url>,
        nym_api_endpoints: Vec<Url>,
    ) -> Self {
        Self {
            user_chosen_gateway,
            nyxd_endpoints,
            nym_api_endpoints,
            debug_config: DebugConfig::default(),
        }
    }
}
