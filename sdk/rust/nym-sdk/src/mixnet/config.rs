use nym_client_core::config::DebugConfig;
use nym_network_defaults::NymNetworkDetails;

/// Config struct for [`crate::mixnet::MixnetClient`]
pub struct Config {
    /// If the user has explicitly specified a gateway.
    pub user_chosen_gateway: Option<String>,

    pub network_details: NymNetworkDetails,

    /// Flags controlling all sorts of internal client behaviour.
    /// Changing these risk compromising network anonymity!
    pub debug_config: DebugConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_chosen_gateway: Default::default(),
            network_details: Default::default(),
            debug_config: Default::default(),
        }
    }
}

impl Config {
    /// Creates a new [`Config`].
    pub fn new(
        user_chosen_gateway: Option<String>,
        network_details: Option<NymNetworkDetails>,
    ) -> Self {
        Self {
            user_chosen_gateway,
            network_details: network_details.unwrap_or_default(),
            debug_config: DebugConfig::default(),
        }
    }
}
