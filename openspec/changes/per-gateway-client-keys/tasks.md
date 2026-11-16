## 1. Registration store: the generation id

- [x] 1.1 Add a `GenerationId` type over 16 bytes to `common/client-core/gateways-storage`, with an all-zero value for a first registration and a constructor that draws 16 bytes from the OS random generator
- [ ] 1.2 Add an optional generation id to `RemoteGatewayDetails` and `RawRemoteGatewayDetails`, and take it in `GatewayDetails::new_remote`, with an accessor on `GatewayDetails` that returns none for a custom registration
- [ ] 1.3 Add the single sqlite migration for this change in `fs_gateways_migrations`: a nullable 16-byte column on `remote_gateway_details`, left empty on every existing row
- [ ] 1.4 Read and write the column in the on-disk backend's queries, and carry the field in the in-memory backend
- [ ] 1.5 Carry the field in the wasm registration store (`common/wasm/client-core/src/storage`) as a defaulted optional field, treating a stored registration without it as legacy
- [ ] 1.6 Add an operation to `GatewaysDetailsStore` that replaces the shared key and generation id of an existing remote registration in place, in the manner of `update_gateway_published_data`: a single update in the on-disk backend, plus the in-memory backend, the wasm store and the SDK's custom-storage example
- [ ] 1.7 Test the on-disk backend against an in-memory database opened without a database key: a registration round-trips with no generation id, with the zero id and with a random id, and the replace operation changes the shared key and generation id while the gateway stays registered and active

## 2. Per-gateway key derivation

- [ ] 2.1 Add the derivation to `common/client-core/src/client/key_manager`: HKDF over BLAKE3 through `nym_crypto::hkdf::extract_then_expand`, the root identity private key as input key material, the salt label, and key-type label, gateway identity public key and generation id as info, building an ed25519 keypair from the identity output and an x25519 keypair from the other
- [ ] 2.2 Fix the labels as constants beside the derivation: salt `nym-client-gateway-keys-v1`, info labels `identity` and `diffie-hellman`
- [ ] 2.3 Add a known-answer test for a fixed root key, gateway key and generation id, using deterministic keys from `nym_test_utils`, with a comment saying that a changed output strands existing registrations
- [ ] 2.4 Test that changing the gateway key, the generation id or the key type each changes the derived key, and that the root x25519 key has no influence on either output

## 3. The keys in use with a gateway

- [ ] 3.1 Add `GatewayClientKeys` to the key manager, holding the identity keypair, the x25519 keypair and the acknowledgement key in use with one gateway, with no way to persist it to a key store
- [ ] 3.2 Add a method on `ClientKeys` that resolves the `GatewayClientKeys` for a registration: derived identity and x25519 keys when the registration carries a generation id, the root keys when it does not or when it is a custom registration, and the root acknowledgement key in every case
- [ ] 3.3 Make `InitialisationResult` carry a `GatewayClientKeys`; in `setup_new_gateway`, register a remote gateway with the keys derived at the zero generation id and store that id with the registration, and keep registering a custom gateway with the root keys
- [ ] 3.4 Return the resolved keys from `use_loaded_gateway_details`, and convert `GatewaySetup::ReuseConnection` and `try_reuse_connection` to carry `GatewayClientKeys`, leaving the commented-out call in the wasm client as it is
- [ ] 3.5 Follow the new type in the base client and in the other readers of the initialisation result: `common/wasm/client-core`, `wasm/client`, `common/socks5-client-core`, `service-providers/network-requester` and `sdk/rust/nym-sdk`
- [ ] 3.6 Test the resolution: a legacy registration yields the root keys, a derived registration yields keys that differ from the root and match the derivation, a custom registration yields the root keys, and two gateways yield different identity and x25519 keys

## 4. Refresh on connect

- [ ] 4.1 Add a refresh path to `common/client-core/src/init` beside `setup_new_gateway`: for a gateway that already has a stored registration, choose the next generation id (the zero id for a legacy registration, a random id for a derived one), register under the keys derived from it, and only then replace the stored registration through the new store operation
- [ ] 4.2 When the registration of a refresh fails, return an error from the connect and leave the stored registration untouched, with no fallback to it
- [ ] 4.3 Add the one-shot `refresh_gateway_keys` flag to the SDK's `MixnetClientBuilder`, and in `setup_gateway` take the refresh path where the stored registration for the chosen gateway would otherwise be loaded; with no stored registration the flag changes nothing
- [ ] 4.4 Surface the flag through the `nym-registration-client` builder
- [ ] 4.5 Test the choice of the next generation id: zero for a legacy registration, a non-zero random value for a derived one

## 5. Full reset

- [ ] 5.1 Add `reset_client_identity` to the SDK, available on a client that is not connected and working from `StoragePaths` with its database key: generate and store new random root keys, remove every stored gateway registration, and clear the reply store's reply SURBs and reply keys with its existing delete operations, without contacting a gateway and without opening or modifying the credential store
- [ ] 5.2 Surface the reset through `nym-registration-client`
- [ ] 5.3 Test the reset: the root identity key differs afterwards, no registration is left and the reply store holds no reply SURBs or reply keys

## 6. SDK surface

- [ ] 6.1 Remove `identity_keypair()`, `sign()` and `sign_text()` from the SDK's `MixnetClient`, and the field that only served them
- [ ] 6.2 Remove or rewrite any example, binding or doc text in the repository that uses those three methods

## 7. Remove derivation material

- [ ] 7.1 Remove `DerivationMaterial` from `common/crypto/src/hkdf.rs`, `ClientKeys::from_master_key` with its tests in `key_manager/test.rs`, and the derivation material argument of `initialise_keys_and_gateway`
- [ ] 7.2 Remove `with_derivation_material` from client-core's base client builder and from the SDK's `MixnetClientBuilder`, with the fields behind them
- [ ] 7.3 Remove `ed25519::KeyPair::from_secret`, the keypair's `index` field and `fake_index`
- [ ] 7.4 Run the workspace tests and update any value that a test pinned from a seeded generator and that changed because `KeyPair::new` no longer draws the index

## 8. Ecash user key and the bandwidth fetcher

- [ ] 8.1 Add the mnemonic helper to `nym-credentials`: take a `&bip39::Mnemonic`, compute SHA-256 over the label `nym-ecash-user-key-from-mnemonic-v1` and the mnemonic entropy in a zeroizing buffer, and return the `KeyPairUser` from the existing seeded key generation
- [ ] 8.2 Add a known-answer test for the helper on a fixed mnemonic, with a comment saying that a changed output changes the ecash key of every self-funding client
- [ ] 8.3 Change `NyxdCredentialFetcher::new` to take a `&bip39::Mnemonic` and derive the keypair with the helper, remove the constructor that takes seed bytes, and leave `new_with_keypair` using its keypair unchanged
- [ ] 8.4 Give the recovery-only fetcher a construction path that takes no ecash key and no placeholder keypair, and update `recover_ticket_book` in nym-cli to use it
- [ ] 8.5 In nym-sdk, make `create_bandwidth_client` and `BandwidthAcquireClient` pass the mnemonic to the fetcher and stop loading the client keys
- [ ] 8.6 In nym-cli's `issue_ticket_book`, pass the paying mnemonic down to the client-config path and stop reading the client's identity key file there; in the output-file path build the keypair from the explicit secret and use `new_with_keypair`
- [ ] 8.7 In `nym-sdk-session`, pass the mnemonic to the fetcher and delete `derive_client_id`
- [ ] 8.8 Update the fetcher's existing unit test to the new constructor

## 9. Compatibility of existing ticketbooks

- [ ] 9.1 Confirm from the finished diff that nothing under `common/credential-storage` changed and that the serialised forms of `IssuedTicketBook` and `IssuanceTicketBook` are untouched
- [ ] 9.2 Confirm from the finished diff that the spend path (`prepare_for_spending`, the bandwidth controller's ticket preparation) reads no client key, no mnemonic and no fetcher state

## 10. Verification

- [ ] 10.1 Run `cargo fmt`, then build and run the tests of `nym-crypto`, `nym-client-core`, `nym-client-core-gateways-storage`, `nym-credentials`, `nym-bandwidth-fetcher`, `nym-sdk`, `nym-sdk-session`, `nym-registration-client` and `nym-cli-commands`
- [ ] 10.2 Check that `nym-wasm-client-core` and the wasm client still build for their wasm target
- [ ] 10.3 Run `openspec validate per-gateway-client-keys` and fix anything it reports
- [ ] 10.4 Copy the QA checklist from `design.md` into the PR description
