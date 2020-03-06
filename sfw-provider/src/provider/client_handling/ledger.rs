use directory_client::presence::providers::MixProviderClient;
use futures::lock::Mutex;
use sfw_provider_requests::AuthToken;
use sphinx::route::DestinationAddressBytes;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
// Note: you should NEVER create more than a single instance of this using 'new()'.
// You should always use .clone() to create additional instances
pub struct ClientLedger {
    inner: Arc<Mutex<ClientLedgerInner>>,
}

impl ClientLedger {
    pub(crate) fn new() -> Self {
        ClientLedger {
            inner: Arc::new(Mutex::new(ClientLedgerInner(HashMap::new()))),
        }
    }

    pub(crate) async fn verify_token(
        &self,
        auth_token: &AuthToken,
        client_address: &DestinationAddressBytes,
    ) -> bool {
        match self.inner.lock().await.0.get(client_address) {
            None => false,
            Some(expected_token) => expected_token == auth_token,
        }
    }

    pub(crate) async fn insert_token(
        &mut self,
        auth_token: AuthToken,
        client_address: DestinationAddressBytes,
    ) -> Option<AuthToken> {
        self.inner.lock().await.0.insert(client_address, auth_token)
    }

    pub(crate) async fn remove_token(
        &mut self,
        client_address: &DestinationAddressBytes,
    ) -> Option<AuthToken> {
        self.inner.lock().await.0.remove(client_address)
    }

    pub(crate) async fn current_clients(&self) -> Vec<MixProviderClient> {
        self.inner
            .lock()
            .await
            .0
            .keys()
            .map(|client_address| client_address.to_base58_string())
            .map(|pub_key| MixProviderClient { pub_key })
            .collect()
    }

    #[allow(dead_code)]
    pub(crate) fn load(_file: PathBuf) -> Self {
        // TODO: actual loading,
        // temporarily just create a new one
        Self::new()
    }

    #[allow(dead_code)]
    pub(crate) async fn save(&self, _file: PathBuf) -> io::Result<()> {
        unimplemented!()
    }
}

struct ClientLedgerInner(HashMap<DestinationAddressBytes, AuthToken>);
