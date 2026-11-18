## 1. Registration store: the generation id

- [x] 1.1 Add a `GenerationId` type over 16 bytes to `common/client-core/gateways-storage`, with an all-zero value for the LP client (`GenerationId::LP`, documented as never stored) and a constructor that draws 16 bytes from a caller-supplied `CryptoRng`
- [x] 1.2 Add an optional generation id to `RemoteGatewayDetails` and `RawRemoteGatewayDetails`, and take it in `GatewayDetails::new_remote`, with an accessor on `GatewayDetails` that returns none for a custom registration
- [x] 1.3 Add the single sqlite migration for this change in `fs_gateways_migrations`: a nullable 16-byte column on `remote_gateway_details`, left empty on every existing row
- [x] 1.4 Read and write the column in the on-disk backend's queries, and carry the field in the in-memory backend
- [x] 1.5 Carry the field in the wasm registration store (`common/wasm/client-core/src/storage`) as a defaulted optional field, treating a stored registration without it as legacy
- [x] 1.6 Add an operation to `GatewaysDetailsStore` that replaces the shared key and generation id of an existing remote registration in place, in the manner of `update_gateway_published_data`: a single update in the on-disk backend, plus the in-memory backend, the wasm store and the SDK's custom-storage example
- [x] 1.7 Test the on-disk backend against an in-memory database opened without a database key: a registration round-trips with no generation id and with a random id, and the replace operation changes the shared key and generation id while the gateway stays registered and active

## 2. Per-gateway key derivation

- [x] 2.1 Add the derivation to `common/client-core/src/client/key_manager`: HKDF over BLAKE3, extracted once from the root identity private key and the salt label into a zeroized pseudorandom key held by a derivation struct, then expanded per key type with the key-type label, gateway identity public key and generation id as info, building an ed25519 keypair from the identity output and an x25519 keypair from the other
- [x] 2.2 Fix the labels as constants beside the derivation: salt `nym-client-gateway-keys-v1`, info labels `identity` and `diffie-hellman`
- [x] 2.3 Add a known-answer test for a fixed root key, gateway key and generation id, built from fixed bytes (not `nym_test_utils`, whose seeded keys change when 7.3 removes the keypair `index` field) and checked against an independent computation from the spec, with a comment saying that a changed output strands existing registrations
- [x] 2.4 Test that changing the gateway key, the generation id or the key type each changes the derived key; the root x25519 key cannot influence either output because the derivation does not take it

## 3. The keys in use with a gateway

- [x] 3.1 Add `GatewayClientKeys` to the key manager, holding the identity keypair, the x25519 keypair and the acknowledgement key in use with one gateway, with no way to persist it to a key store
- [x] 3.2 Add a method on `ClientKeys` that resolves the `GatewayClientKeys` from a gateway identity and an optional generation id: derived identity and x25519 keys when an id is given, the root keys when none is (a legacy or custom registration supplies none), and the root acknowledgement key in every case
- [x] 3.3 Make `InitialisationResult` carry a `GatewayClientKeys` in a field named `gateway_keys`; in `setup_new_gateway`, register a remote gateway with the keys derived from a generation id drawn with `os_rng()` and store that id with the registration, and keep registering a custom gateway with the root keys; rename `GenerationId::INITIAL` to `GenerationId::LP`, documenting why the LP client uses a fixed id and that no stored registration carries it
- [x] 3.4 Return the resolved keys from `use_loaded_gateway_details`, through `InitialisationResult::new_loaded`, which resolves them from the registration it is given, and convert `GatewaySetup::ReuseConnection` and `try_reuse_connection` to carry `GatewayClientKeys`, leaving the commented-out call in the wasm client as it is
- [x] 3.5 Follow the new type in the base client and in the other readers of the initialisation result: `common/wasm/client-core`, `wasm/client`, `common/socks5-client-core`, `service-providers/network-requester` and `sdk/rust/nym-sdk`; only the base client needed a change (made with 3.3), since the other readers take only the address, the gateway id or the registration from it
- [x] 3.6 In the LP client (`common/client-core/src/client/base_client/lp_client.rs`), once the gateway is chosen, resolve the keys for it at `GenerationId::LP` and use them for the address, the dialer's registration and the data plane's decryption key; add a comment on the dialer's `request()` that a session with any further gateway needs keys derived for that gateway first
- [x] 3.7 Test the resolution: a legacy registration yields the root keys, a derived registration yields keys that differ from the root and match the derivation, a custom registration yields the root keys, and two gateways yield different identity and x25519 keys

## 4. Refresh on connect

- [x] 4.1 Add a refresh path to `common/client-core/src/init` beside `setup_new_gateway`, selected by a new `GatewaySetup::RefreshKeys` variant: for a gateway that already has a stored registration, legacy or derived, draw a new generation id with `os_rng()`, register under the keys derived from it over the listeners stored with the registration, and only then replace the stored registration through the new store operation; a custom registration is refused with an error
- [x] 4.2 When the registration of a refresh fails, return an error from the connect and leave the stored registration untouched, with no fallback to it
- [x] 4.3 Add the one-shot `refresh_gateway_keys` flag to the SDK's `MixnetClientBuilder`, and in `setup_gateway` take the refresh path where the stored registration for the chosen gateway would otherwise be loaded; with no stored registration the flag changes nothing
- [x] 4.4 Fail a connect over LP (`connect_to_lp`) when `refresh_gateway_keys` is set, before any gateway is dialled
- [x] 4.5 Surface the flag through the `nym-registration-client` builder (a defaulted `BuilderConfig` field passed to the mixnet client it connects; the LP registration path builds no mixnet client and does not use it)
- [x] 4.6 Test that a connect over LP with the flag set fails before any gateway is dialled

## 5. Full reset

- [x] 5.1 Add `reset_client_identity` to the SDK, available on a client that is not connected and working from `StoragePaths` with its database key: generate and store new random root keys, remove every stored gateway registration, and clear the reply store's reply SURBs and reply keys with its existing delete operations, without contacting a gateway and without opening or modifying the credential store (a method on `StoragePaths`; registrations go first and the keys last, so an interrupted reset leaves usable state and can be run again; the reply store is loaded directly and only when its database exists, since the usual setup would create it with default thresholds; the on-disk `remove_gateway_details` now deletes the details rows before the registered row they reference)
- [x] 5.2 Surface the reset through `nym-registration-client` (a free `reset_client_identity(data_path, database_key)` using the same storage layout as the mixnet client it builds)
- [x] 5.3 Test the reset: the root identity key differs afterwards, no registration is left and the reply store holds no reply SURBs or reply keys (the SDK test stores a reply key through the backend trait; a unit test of the reply store's `delete_all_reply_data` covers reply SURBs with raw rows)

## 6. SDK surface

- [x] 6.1 Remove `identity_keypair()`, `sign()` and `sign_text()` from the SDK's `MixnetClient`, and the field that only served them (with the client-core fields that only fed it: `BaseClient.identity_keys` and `LpBaseClient`'s `identity_keys` field and accessor)
- [x] 6.2 Remove or rewrite any example, binding or doc text in the repository that uses those three methods (none did: no example, FFI binding or doc page calls them on a `MixnetClient`)

## 7. Remove derivation material

- [x] 7.1 Remove `DerivationMaterial` from `common/crypto/src/hkdf.rs`, `ClientKeys::from_master_key` with its tests in `key_manager/test.rs`, and the derivation material argument of `initialise_keys_and_gateway` (and of `initialise_keys`, with `ClientCoreError::HkdfDerivationError` and client-core's `rand_chacha` dependency, which only these served)
- [x] 7.2 Remove `with_derivation_material` from client-core's base client builder, the LP client builder (`LpBaseClientBuilder`) and the SDK's `MixnetClientBuilder`, with the fields behind them
- [x] 7.3 Remove `ed25519::KeyPair::from_secret`, the keypair's `index` field and `fake_index` (keypairs the wasm client stored with an `index` still load, since the derived `Deserialize` ignores unknown fields; a comment on `KeyPair` says so)
- [x] 7.4 Run the workspace tests and update any value that a test pinned from a seeded generator and that changed because `KeyPair::new` no longer draws the index (one: the three keys in `nym-crypto`'s `vec_bs58_ed25519_pubkey_json`; the contracts workspace, which patches in the local `nym-crypto`, passed unchanged)

## 8. Ecash user key and the bandwidth fetcher

- [x] 8.1 Add the mnemonic helper to `nym-credentials`: take a `&bip39::Mnemonic`, compute SHA-256 over the label `nym-ecash-user-key-from-mnemonic-v1` and the mnemonic entropy in a zeroizing buffer, and return the `KeyPairUser` from the existing seeded key generation (`ecash_keypair_from_mnemonic` in `ecash/user_key.rs`, exported from the crate root; `bip39` through the validator client's re-export, `sha2` added as a direct dependency)
- [x] 8.2 Add a known-answer test for the helper on a fixed mnemonic, with a comment saying that a changed output changes the ecash key of every self-funding client (the mnemonic is built from fixed entropy bytes; the seed is checked against an independent SHA-256, and the public key is recorded from the implementation, since it comes from the existing seeded ecash key generation)
- [x] 8.3 Change `NyxdCredentialFetcher::new` to take a `&bip39::Mnemonic` and derive the keypair with the helper, remove the constructor that takes seed bytes, and leave `new_with_keypair` using its keypair unchanged (the seed-bytes constructor was `new` itself, so its last argument changed from the seed bytes to the mnemonic)
- [x] 8.4 Give the recovery-only fetcher a construction path that takes no ecash key and no placeholder keypair (both fetchers are built over a private part that holds no key: the chain client, the pending-deposit store, the signing-data fetchers and the recovery steps; only `NyxdCredentialFetcher` adds the ecash keypair and the deposit lock; `NyxdRecoveryFetcher::new` kept its signature, so `recover_ticket_book` in nym-cli needed no change)
- [x] 8.5 In nym-sdk, make `create_bandwidth_client` and `BandwidthAcquireClient` pass the mnemonic to the fetcher and stop loading the client keys (`create_bandwidth_client` keeps its `String` parameter; `BandwidthAcquireClient::new` parses it once, gives a clone to the chain client and a reference to the fetcher; nym-sdk's `zeroize` dependency, used only by the removed key load, is gone)
- [x] 8.6 In nym-cli's `issue_ticket_book`, pass the paying mnemonic down to the client-config path and stop reading the client's identity key file there; in the output-file path build the keypair from the explicit secret and use `new_with_keypair` (`execute` takes the mnemonic and the network details and builds the paying client itself, cloning the mnemonic only on the client-config path, where the fetcher needs it too; the mnemonic comes from `signing_mnemonic`, the rule `create_signing_client` uses, so the same mnemonic pays and derives the key; `--key-passphrase`, both `try_get_private_id_key` and `nym-pemstore`'s `encryption` feature, which served only the key file, are gone)
- [x] 8.7 In `nym-sdk-session`, pass the mnemonic to the fetcher and delete `derive_client_id` (the chain client takes a clone and the fetcher borrows the mnemonic; the crate's `sha2` and `zeroize` dependencies, used only by `derive_client_id`, are gone)
- [x] 8.8 Update the fetcher's existing unit test to the new constructor (it exercises only the pending store and the pruning, so it builds the key-free `FetcherCore` from 8.4 directly; the `EcashSigningClient` and `EcashQueryClient` mocks, which no call in it needed, are gone)

## 9. Reset: a reply store that cannot be loaded

- [x] 9.1 In `StoragePaths::reset_client_identity`, when the reply store fails to load with `IncompleteDataFlush` (a flush cut short or never made, the states in which a client start archives the store and begins a fresh one), delete the store's database and journal files and continue; every other load failure, a wrong database key included, still fails the reset; add a sentence on this to design Decision 8
- [x] 9.2 Test the reset on a reply store that was created and never flushed: assert first that loading it fails with `IncompleteDataFlush`, then that the reset succeeds, the store's files are gone and the root identity key changed

## 10. Compatibility of existing ticketbooks

- [ ] 10.1 Confirm from the finished diff that nothing under `common/credential-storage` changed and that the serialised forms of `IssuedTicketBook` and `IssuanceTicketBook` are untouched
- [ ] 10.2 Confirm from the finished diff that the spend path (`prepare_for_spending`, the bandwidth controller's ticket preparation) reads no client key, no mnemonic and no fetcher state

## 11. Verification

- [ ] 11.1 Run `cargo fmt`, then build and run the tests of `nym-crypto`, `nym-client-core`, `nym-client-core-gateways-storage`, `nym-credentials`, `nym-bandwidth-fetcher`, `nym-sdk`, `nym-sdk-session`, `nym-registration-client` and `nym-cli-commands`
- [ ] 11.2 Check that `nym-wasm-client-core` and the wasm client still build for their wasm target
- [ ] 11.3 Run `openspec validate per-gateway-client-keys` and fix anything it reports
- [ ] 11.4 Copy the QA checklist from `design.md` into the PR description
