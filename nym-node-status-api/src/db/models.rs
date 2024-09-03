// TODO dz: considerstructs with named fields, instead of tuple type aliases

pub(crate) type GatewayRecord = (
    String, // gateway_identity_key
    bool,   // bonded
    bool,   // blacklisted
    // TODO originally this was String, but could be empty
    Option<String>, // self_described
    Option<String>, // explorer_pretty_bond
    i64,            // last_updated_utc
    u8,             // gateway_performance
);

pub(crate) type MixnodeRecord = (
    u32,            // mix_id
    String,         // identity_key
    bool,           // bonded
    i64,            // total_stake
    String,         // host
    u16,            // http_port
    bool,           // blacklisted
    String,         // full_details
    Option<String>, // self_described
    i64,            // last_updated_utc
    bool,           // is_dp_delegatee
);
