use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentWatcherConfig {
    pub watchers: Vec<PaymentWatcherEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentWatcherEntry {
    pub id: String,
    pub description: Option<String>,
    pub webhook_url: String,
    pub watch_for_transfer_recipient_accounts: Option<Vec<AccountId>>,
    pub watch_for_chain_message_types: Option<Vec<String>>,
    pub authentication: Option<HttpAuthenticationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpAuthenticationOptions {
    AuthorizationBearerToken { token: String },
}
