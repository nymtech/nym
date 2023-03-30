use cosmwasm_std::{Addr, Coin, DepsMut, Storage};
use cw_controllers::Admin;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::error::Result;

const CONFIG_KEY: &str = "config";
const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

const ADMIN_KEY: &str = "admin";
const ADMIN: Admin = Admin::new(ADMIN_KEY);

pub(crate) fn set_admin(deps: DepsMut<'_>, admin: Addr) -> Result<()> {
    Ok(ADMIN.set(deps, Some(admin))?)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct Config {
    pub deposit_required: Coin,
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
