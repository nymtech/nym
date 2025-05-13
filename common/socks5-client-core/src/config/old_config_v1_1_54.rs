use super::Config;
pub use nym_client_core::config::old_config_v1_1_54::ConfigV1_1_54 as BaseClientConfigV1_1_54;
use serde::{Deserialize, Serialize};

use super::Socks5;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_54 {
    #[serde(flatten)]
    pub base: BaseClientConfigV1_1_54,

    pub socks5: Socks5,
}

impl From<ConfigV1_1_54> for Config {
    fn from(value: ConfigV1_1_54) -> Self {
        Config {
            base: value.base.into(),
            socks5: value.socks5.into(),
        }
    }
}
