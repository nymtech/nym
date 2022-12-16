use crate::support::caching::Cache;

pub(crate) struct CirculatingSupplyCacheData {
    pub(crate) circulating_supply: Cache<String>,
}

impl CirculatingSupplyCacheData {
    pub fn new() -> CirculatingSupplyCacheData {
        CirculatingSupplyCacheData {
            circulating_supply: Cache::default(),
        }
    }
}
