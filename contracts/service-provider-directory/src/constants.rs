// We limit the these for simplicity and to avoid having to deal with paging.
pub const MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER: u32 = 100;
pub const MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS: u32 = 100;

pub const SERVICE_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const SERVICE_MAX_RETRIEVAL_LIMIT: u32 = 150;

// Storage keys
pub const CONFIG_KEY: &str = "config";
pub const ADMIN_KEY: &str = "admin";
pub const SERVICE_ID_COUNTER_KEY: &str = "sidc";

pub const SERVICES_PK_NAMESPACE: &str = "sernames";
pub const SERVICES_ANNOUNCER_IDX_NAMESPACE: &str = "serown";
pub const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";
