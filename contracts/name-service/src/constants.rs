// We limit the these for simplicity and to avoid having to deal with paging.
pub const MAX_NUMBER_OF_NAMES_PER_OWNER: u32 = 100;
pub const MAX_NUMBER_OF_NAMES_FOR_ADDRESS: u32 = 100;

pub const NAME_DEFAULT_RETRIEVAL_LIMIT: u32 = 100;
pub const NAME_MAX_RETRIEVAL_LIMIT: u32 = 150;

// Storage keys
pub const CONFIG_KEY: &str = "config";
pub const ADMIN_KEY: &str = "admin";
pub const NAME_ID_COUNTER_KEY: &str = "nidc";

pub const NAMES_PK_NAMESPACE: &str = "nanames";
pub const NAMES_OWNER_IDX_NAMESPACE: &str = "naowner";
pub const NAMES_ADDRESS_IDX_NAMESPACE: &str = "naaddress";
pub const NAMES_NAME_IDX_NAMESPACE: &str = "naname";

pub const SIGNING_NONCES_NAMESPACE: &str = "nasn";
