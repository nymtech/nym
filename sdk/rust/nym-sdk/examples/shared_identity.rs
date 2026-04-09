/// Proof-of-concept: two mixnet clients sharing the same identity,
/// where the second client reuses the first client's gateway registration
/// (shared key) without re-registering.
///
/// Could be used in the NS API optimization where
/// multiple gateway probes share a single identity and registration cache.
///
/// Run from the sdk/rust/nym-sdk directory:
///   cargo run --example shared_identity
///
/// Flow:
///   - Client-A registers with a gateway (DH handshake → shared key)
///   - Client-B uses the SAME identity and the SAME shared key
///   - Client-B authenticates (no DH) and gets connected
///   - Both clients get the same Nym address (proves shared identity works)
use nym_client_core::client::base_client::storage::gateways_storage::{
    GatewayDetails, GatewaysDetailsStore, InMemGatewaysDetails,
};
use nym_client_core::client::base_client::storage::{Ephemeral, MixnetClientStorage};
use nym_client_core::client::key_manager::persistence::{InMemEphemeralKeys, KeyStore};
use nym_client_core::client::replies::reply_storage;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_credentials_interface::TicketType;
use nym_gateway_requests::shared_key::SharedSymmetricKey;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use rand::rngs::OsRng;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    nym_bin_common::logging::setup_tracing_logger();

    // export ENVIRONMENT=sandbox
    let sandbox_network = mixnet::NymNetworkDetails::new_from_env();

    println!("=== Phase 1: Client-A registers with a gateway ===\n");

    // create client-A with ephemeral storage (will register fresh)
    let storage_a = Ephemeral::default();

    // save a handle to the stores so we can extract data after connection
    let keys_store_a = storage_a.key_store().clone();
    let gw_store_a = storage_a.gateway_details_store().clone();

    let client_a_builder = mixnet::MixnetClientBuilder::new_with_storage(storage_a)
        .network_details(sandbox_network.clone())
        .enable_credentials_mode()
        .build()?;

    let mnemonic = std::env::var("SANDBOX_MNEMONIC").expect("Set mnemonic in the env variable");
    let bandwidth_client_a = client_a_builder
        .create_bandwidth_client(mnemonic, TicketType::V1MixnetEntry)
        .await?;

    // get a bandwidth credential for the mixnet_client
    bandwidth_client_a.acquire().await?;

    // connect: this triggers gateway registration (DH handshake)
    let mut client_a = client_a_builder.connect_to_mixnet().await?;
    let address_a = *client_a.nym_address();
    println!("Client-A address: {address_a}");

    // extract the identity keys and gateway registration from client-A
    let client_keys = keys_store_a.load_keys().await?;
    let active_gw = gw_store_a.active_gateway().await?;
    let registration = active_gw
        .registration
        .expect("client-A should have an active gateway registration");

    // extract the shared key and gateway info
    let shared_key_bytes = registration
        .details
        .shared_key()
        .expect("remote gateway should have a shared key")
        .to_bytes();
    let gateway_id = registration.details.gateway_id();
    let published_data = match &registration.details {
        GatewayDetails::Remote(r) => r.published_data.clone(),
        _ => panic!("expected remote gateway"),
    };

    println!("Gateway ID: {}", gateway_id.to_base58_string());
    println!(
        "Shared key (first 8 bytes): {:02x?}",
        &shared_key_bytes[..8]
    );
    println!("Gateway listener: {}", published_data.listeners.primary);

    // send a self-ping to prove client-A works
    client_a
        .send_plain_message(address_a, "hello from client-A")
        .await?;
    println!("\nClient-A sent self-ping...");
    if let Some(msgs) = client_a.wait_for_messages().await {
        for m in &msgs {
            println!("Client-A received: {}", String::from_utf8_lossy(&m.message));
        }
    }

    // disconnect client-A (don't ForgetMe: keep registration alive)
    client_a.disconnect().await;
    println!("\nClient-A disconnected (gateway keeps registration)\n");

    // ================================================================
    println!("=== Phase 2: Client-B reuses Client-A's identity + registration ===\n");

    // reconstruct the shared key from raw bytes
    let shared_key = SharedSymmetricKey::try_from_bytes(&shared_key_bytes)?;

    // build a pre-populated gateway registration
    let gw_details = GatewayDetails::new_remote(gateway_id, Arc::new(shared_key), published_data);
    let gw_registration = gw_details.into();

    // create a new ephemeral storage and pre-populate it
    let keys_store_b = InMemEphemeralKeys::new(&mut OsRng);
    // inject the SAME identity keys from client-A
    keys_store_b.store_keys(&client_keys).await?;

    let gw_store_b = InMemGatewaysDetails::default();
    // inject the gateway registration (shared key) from client-A
    gw_store_b.store_gateway_details(&gw_registration).await?;
    gw_store_b
        .set_active_gateway(&gateway_id.to_base58_string())
        .await?;

    // wrap into a custom storage that the SDK can use
    let storage_b = PrePopulatedStorage {
        key_store: keys_store_b,
        reply_store: reply_storage::Empty::default(),
        credential_store: EphemeralCredentialStorage::default(),
        gateway_details_store: gw_store_b,
    };

    // Build client-B with the pre-populated storage
    // request_gateway forces the builder to use our pre-registered gateway
    let client_b_builder = mixnet::MixnetClientBuilder::new_with_storage(storage_b)
        .request_gateway(gateway_id.to_base58_string())
        .network_details(sandbox_network)
        .enable_credentials_mode()
        .build()?;

    // === what we'd do when claiming bandwidth: ===
    // but we do NOT do this because we want to reuse registration from client A
    // let bandwidth_client_b = client_b_builder
    //     .create_bandwidth_client(mnemonic, TicketType::V1MixnetEntry)
    //     .await?;
    // bandwidth_client_b.acquire().await?;

    // Connect: this should AUTHENTICATE (not register) because the
    // gateway details store already has a registration for this gateway
    println!("Client-B connecting (should authenticate, not register)...");
    let mut client_b = client_b_builder.connect_to_mixnet().await?;
    let address_b = *client_b.nym_address();
    println!("Client-B address: {address_b}");

    // Verify: both clients get the same Nym address
    if address_a == address_b {
        println!("\n*** SUCCESS: Both clients have the SAME Nym address ***");
        println!("*** Client-B authenticated using Client-A's shared key ***");
    } else {
        println!("\n*** UNEXPECTED: Addresses differ ***");
        println!("  A: {address_a}");
        println!("  B: {address_b}");
    }

    // send a self-ping to prove client-B works
    client_b
        .send_plain_message(address_b, "hello from client-B")
        .await?;
    println!("\nClient-B sent self-ping...");
    if let Some(msgs) = client_b.wait_for_messages().await {
        for m in &msgs {
            println!("Client-B received: {}", String::from_utf8_lossy(&m.message));
        }
    }

    client_b.disconnect().await;
    println!("\nClient-B disconnected. Done!");

    Ok(())
}

// A wrapper around in-memory stores to implement MixnetClientStorage.
// This is the pattern an NS Agent would use to create probes with
// pre-populated identity and gateway registration.
#[derive(Clone)]
struct PrePopulatedStorage {
    key_store: InMemEphemeralKeys,
    reply_store: reply_storage::Empty,
    credential_store: EphemeralCredentialStorage,
    gateway_details_store: InMemGatewaysDetails,
}

impl MixnetClientStorage for PrePopulatedStorage {
    type KeyStore = InMemEphemeralKeys;
    type ReplyStore = reply_storage::Empty;
    type CredentialStore = EphemeralCredentialStorage;
    type GatewaysDetailsStore = InMemGatewaysDetails;

    fn into_runtime_stores(
        self,
    ) -> (
        Self::ReplyStore,
        Self::CredentialStore,
        Self::GatewaysDetailsStore,
    ) {
        (
            self.reply_store,
            self.credential_store,
            self.gateway_details_store,
        )
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
