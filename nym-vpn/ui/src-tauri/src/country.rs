use once_cell::sync::Lazy;

use crate::states::app::Country;

// TODO use hardcoded country list for now
pub static COUNTRIES: Lazy<Vec<Country>> = Lazy::new(|| {
    vec![
        Country {
            name: "France".to_string(),
            code: "FR".to_string(),
        },
        Country {
            name: "Germany".to_string(),
            code: "DE".to_string(),
        },
        Country {
            name: "Ireland".to_string(),
            code: "IE".to_string(),
        },
        Country {
            name: "Japan".to_string(),
            code: "JP".to_string(),
        },
        Country {
            name: "United Kingdom".to_string(),
            code: "GB".to_string(),
        },
    ]
});

pub static DEFAULT_NODE_LOCATION: Lazy<Country> = Lazy::new(|| Country {
    code: "FR".to_string(),
    name: "France".to_string(),
});
