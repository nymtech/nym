use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PaymentWatchersConfig {
    pub watchers: Vec<PaymentWatcherConfig>,
}

impl PaymentWatchersConfig {
    pub fn is_being_watched(&self, account: &str) -> bool {
        self.watchers.iter().any(|watcher| {
            watcher
                .watch_for_transfer_recipient_accounts
                .iter()
                .any(|acc| acc.as_ref() == account)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentWatcherConfig {
    pub id: String,
    pub description: Option<String>,
    pub webhook_url: String,
    pub watch_for_transfer_recipient_accounts: Vec<AccountId>,
    pub watch_for_chain_message_types: Vec<String>,
    pub authentication: Option<HttpAuthenticationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpAuthenticationOptions {
    AuthorizationBearerToken { token: String },
}
