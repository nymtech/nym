use once_cell::sync::Lazy;

use crate::states::app::Country;

// TODO use hardcoded country list for now
pub static COUNTRIES: Lazy<Vec<Country>> = Lazy::new(|| {
    vec![
        Country {
            name: "Ireland".to_string(),
            code: "IE".to_string(),
        },
        Country {
            name: "Germany".to_string(),
            code: "DE".to_string(),
        },
        Country {
            name: "Japan".to_string(),
            code: "JP".to_string(),
        },
        Country {
            name: "Great Britain".to_string(),
            code: "GB".to_string(),
        },
    ]
});
