use nym_client_core::config::DebugConfig;
use nym_network_defaults::NymNetworkDetails;
use nym_sphinx::params::PacketType;

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

    pub packet_type: PacketType,
}

impl Config {
    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }
}
