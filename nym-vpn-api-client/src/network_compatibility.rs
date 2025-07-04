use serde::{Deserialize, Serialize};
use std::fmt;

use crate::response::NetworkCompatibilityResponse;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkCompatibility {
    pub core: String,
    pub ios: String,
    pub macos: String,
    pub tauri: String,
    pub android: String,
}

impl fmt::Display for NetworkCompatibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "core: {}, ios: {}, macos: {}, tauri: {}, android: {}",
            self.core, self.ios, self.macos, self.tauri, self.android
        )
    }
}

impl From<NetworkCompatibilityResponse> for NetworkCompatibility {
    fn from(response: NetworkCompatibilityResponse) -> Self {
        NetworkCompatibility {
            core: response.core,
            ios: response.ios,
            macos: response.macos,
            tauri: response.tauri,
            android: response.android,
        }
    }
}
