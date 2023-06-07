use nym_client_core::config::DebugConfig;
use nym_network_defaults::NymNetworkDetails;

#[derive(Clone, Debug, Default)]
pub enum KeyMode {
    /// Use existing key files if they exists, otherwise create new ones.
    #[default]
    Keep,
    /// Create new keys, overwriting any potential previously existing keys.
    Overwrite,
}

impl KeyMode {
    pub(crate) fn is_keep(&self) -> bool {
        matches!(self, KeyMode::Keep)
    }
}

/// Config struct for [`crate::mixnet::MixnetClient`]
#[derive(Default)]
pub struct Config {
    /// If the user has explicitly specified a gateway.
    pub user_chosen_gateway: Option<String>,

    /// Determines how to handle existing key files found.
    pub key_mode: KeyMode,

    /// The details of the network we're using. It defaults to the mainnet network.
    pub network_details: NymNetworkDetails,

    /// Whether to attempt to use gateway with bandwidth credential requirement.
    pub enabled_credentials_mode: bool,

    /// Flags controlling all sorts of internal client behaviour.
    /// Changing these risk compromising network anonymity!
    pub debug_config: DebugConfig,
}
