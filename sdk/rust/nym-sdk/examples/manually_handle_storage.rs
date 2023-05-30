use nym_sdk::mixnet::{
    self, EmptyReplyStorage, EphemeralCredentialStorage, KeyManager, KeyStore, MixnetClientStorage,
};
use nym_topology::provider_trait::async_trait;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    // Just some plain data to pretend we have some external storage that the application
    // implementer is using.
    let mock_storage = MockClientStorage::empty();
    let mut mock_gw_storage = MockGatewayConfigStorage::empty();

    let first_run = true;

    let client = if first_run {
        // Create a client without a storage backend
        let mut client = mixnet::MixnetClientBuilder::new_with_storage(mock_storage)
            .build()
            .await
            .unwrap();

        // In this we want to provide our own gateway config struct, and handle persisting this info to disk
        // ourselves (e.g., as part of our own configuration file).
        // during registration, our key storage will be automatically called to persist the keys
        client.register_and_authenticate_gateway().await.unwrap();
        mock_gw_storage.write_config(client.get_gateway_endpoint().unwrap());
        client
    } else {
        let gateway_config = mock_gw_storage.read_config();

        // Create a client with a storage backend, so that our keys could be loaded.
        // This creates the client in a registered state.
        mixnet::MixnetClientBuilder::new_with_storage(mock_storage)
            .registered_gateway(gateway_config)
            .build()
            .await
            .unwrap()
    };

    // Connect to the mixnet, now we're listening for incoming
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send important info up the pipe to a buddy
    client.send_str(*our_address, "hello there").await;

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}

#[allow(unused)]
struct MockGatewayConfigStorage {
    pub gateway_config: Option<mixnet::GatewayEndpointConfig>,
}

impl MockGatewayConfigStorage {
    fn read_config(&self) -> mixnet::GatewayEndpointConfig {
        todo!();
    }

    fn write_config(&mut self, _gateway_config: &mixnet::GatewayEndpointConfig) {
        log::info!("todo");
    }

    fn empty() -> Self {
        Self {
            gateway_config: None,
        }
    }
}

#[allow(unused)]
struct MockClientStorage {
    pub key_store: MockKeyStore,
    pub reply_store: EmptyReplyStorage,
    pub credential_store: EphemeralCredentialStorage,
}

impl MockClientStorage {
    fn empty() -> Self {
        Self {
            key_store: MockKeyStore,
            reply_store: EmptyReplyStorage::default(),
            credential_store: EphemeralCredentialStorage::default(),
        }
    }
}

impl MixnetClientStorage for MockClientStorage {
    type KeyStore = MockKeyStore;
    type ReplyStore = EmptyReplyStorage;
    type CredentialStore = EphemeralCredentialStorage;

    fn into_split(self) -> (Self::KeyStore, Self::ReplyStore, Self::CredentialStore) {
        (self.key_store, self.reply_store, self.credential_store)
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
}

struct MockKeyStore;

#[async_trait]
impl KeyStore for MockKeyStore {
    type StorageError = MyError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        println!("loading stored keys");

        Err(MyError)
    }

    async fn store_keys(&self, _keys: &KeyManager) -> Result<(), Self::StorageError> {
        println!("storing keys");

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
