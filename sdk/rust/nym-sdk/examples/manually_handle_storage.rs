use nym_sdk::mixnet::{
    self, ClientKeys, EmptyReplyStorage, EphemeralCredentialStorage, KeyStore, MixnetClientStorage,
    MixnetMessageSender,
};
use nym_topology::provider_trait::async_trait;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Just some plain data to pretend we have some external storage that the application
    // implementer is using.
    let mock_storage = MockClientStorage::empty();
    let mut client = mixnet::MixnetClientBuilder::new_with_storage(mock_storage)
        .build()
        .unwrap()
        .connect_to_mixnet()
        .await
        .unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send important info up the pipe to a buddy
    client
        .send_plain_message(*our_address, "hello there")
        .await
        .unwrap();

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}

#[allow(unused)]
struct MockClientStorage {
    pub key_store: MockKeyStore,
    pub gateway_details_store: MockGatewayDetailsStore,
    pub reply_store: EmptyReplyStorage,
    pub credential_store: EphemeralCredentialStorage,
}

impl MockClientStorage {
    fn empty() -> Self {
        Self {
            key_store: MockKeyStore,
            gateway_details_store: MockGatewayDetailsStore,
            reply_store: EmptyReplyStorage::default(),
            credential_store: EphemeralCredentialStorage::default(),
        }
    }
}

impl MixnetClientStorage for MockClientStorage {
    type KeyStore = MockKeyStore;
    type ReplyStore = EmptyReplyStorage;
    type CredentialStore = EphemeralCredentialStorage;
    type GatewaysDetailsStore = MockGatewayDetailsStore;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore) {
        (self.reply_store, self.credential_store)
    }

    fn key_store(&self) -> &Self::KeyStore {
        &self.key_store
    }

    fn reply_store(&self) -> &Self::ReplyStore {
        &self.reply_store
    }

    fn credential_store(&self) -> &Self::CredentialStore {
        &self.credential_store
    }

    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore {
        &self.gateway_details_store
    }
}

struct MockKeyStore;

#[async_trait]
impl KeyStore for MockKeyStore {
    type StorageError = MyError;

    async fn load_keys(&self) -> Result<ClientKeys, Self::StorageError> {
        println!("loading stored keys");

        Err(MyError)
    }

    async fn store_keys(&self, _keys: &ClientKeys) -> Result<(), Self::StorageError> {
        println!("storing keys");

        Ok(())
    }
}

struct MockGatewayDetailsStore;

#[async_trait]
impl GatewayDetailsStore for MockGatewayDetailsStore {
    type StorageError = MyError;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError> {
        println!("loading stored gateway details");

        Err(MyError)
    }

    async fn store_gateway_details(
        &self,
        _details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError> {
        println!("storing gateway details");

        Ok(())
    }
}

//
// struct MockReplyStore;
//
// #[async_trait]
// impl ReplyStorageBackend for MockReplyStore {
//     type StorageError = MyError;
//
//     async fn flush_surb_storage(
//         &mut self,
//         _storage: &CombinedReplyStorage,
//     ) -> Result<(), Self::StorageError> {
//         todo!()
//     }
//
//     async fn init_fresh(&mut self, _fresh: &CombinedReplyStorage) -> Result<(), Self::StorageError> {
//         todo!()
//     }
//
//     async fn load_surb_storage(&self) -> Result<CombinedReplyStorage, Self::StorageError> {
//         todo!()
//     }
// }
//
// struct MockCredentialStore;
//
// #[async_trait]
// impl CredentialStorage for MockCredentialStore {
//     type StorageError = MyError;
//
//     async fn insert_coconut_credential(
//         &self,
//         _voucher_value: String,
//         _voucher_info: String,
//         _serial_number: String,
//        _binding_number: String,
//         _signature: String,
//         _epoch_id: String,
//     ) -> Result<(), Self::StorageError> {
//         todo!()
//     }
//
//     async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, Self::StorageError> {
//         todo!()
//     }
//
//     async fn consume_coconut_credential(&self, id: i64) -> Result<(), Self::StorageError> {
//         todo!()
//     }
// }

#[derive(thiserror::Error, Debug)]
#[error("foobar")]
struct MyError;
