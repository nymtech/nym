use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PaymentWatcherConfig {
    pub watchers: Vec<PaymentWatcherEntry>,
}

impl PaymentWatcherConfig {
    pub fn watched_transfer_accounts(&self) -> Vec<&AccountId> {
        self.watchers
            .iter()
            .filter_map(|e| e.watch_for_transfer_recipient_accounts.as_ref())
            .flat_map(|a| a)
            .collect()
    }
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
