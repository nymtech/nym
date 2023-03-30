use cosmwasm_std::{Coin, Storage};
use cw_storage_plus::Item;
use nym_service_provider_directory_common::msg::ConfigResponse;
use serde::{Deserialize, Serialize};

use crate::error::Result;

const CONFIG_KEY: &str = "config";
const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct Config {
    pub deposit_required: Coin,
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        ConfigResponse {
            deposit_required: config.deposit_required,
        }
    }
}

pub(crate) fn save_config(store: &mut dyn Storage, config: &Config) -> Result<()> {
    Ok(CONFIG.save(store, config)?)
}

pub(crate) fn load_config(store: &dyn Storage) -> Result<Config> {
    Ok(CONFIG.load(store)?)
}

/// Return the deposit required to announce a service.
pub(crate) fn deposit_required(store: &dyn Storage) -> Result<Coin> {
    Ok(CONFIG.load(store).map(|config| config.deposit_required)?)
}
