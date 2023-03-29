use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::ServiceId;

// Storage keys
pub const SERVICE_ID_COUNTER_KEY: &str = "sidc";

// Storage
pub const SERVICE_ID_COUNTER: Item<ServiceId> = Item::new(SERVICE_ID_COUNTER_KEY);


/// Generate the next service provider id, store it and return it
pub(crate) fn next_service_id_counter(store: &mut dyn Storage) -> Result<ServiceId> {
    // The first id is 1.
    let id = SERVICE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    SERVICE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

