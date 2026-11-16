## Why

A mixnet client has one ed25519 identity key and presents it to every gateway it registers with, so gateways that compare notes can tell they are serving the same client and can follow it over time. The link is not even private to gateways: each gateway publishes an unsalted SHA-256 of every counted client's identity on its metrics endpoint, and the same client produces the same hash everywhere. A gateway can also link every session of one client, even when the client changes network address and pays with unlinkable tickets, because the identity it sees never changes.

The same identity key is also the seed of the client's ecash key in nym-sdk and nym-cli, which ties bandwidth credentials to network keys for no benefit and would make any reset of those keys change the ecash key too.

## What Changes

- **Per-gateway keys.** For every new registration the client derives a separate ed25519 identity key and x25519 encryption key for that gateway, by HKDF over BLAKE3 keyed with its existing identity private key and bound to the gateway's identity key and a per-gateway generation id. The existing keys stay on disk unchanged and become a private root that new registrations never show to a gateway.
- **Existing registrations are untouched.** A registration made before this change keeps using the root keys directly. Nothing is re-registered on upgrade, so addresses and bandwidth already credited at gateways are kept. Custom registrations, used by the service providers embedded in a node, also keep the root keys.
- **Refreshable keys.** A flag on connect gives the client a fresh identity at the gateway it is connecting to, by drawing a new random generation id and registering again. Nothing refreshes without the flag.
- **Full reset.** An explicit call replaces the root keys and clears the gateway registrations and the reply store, keeping the credential store.
- **BREAKING (nym-sdk):** `MixnetClient::identity_keypair()`, `sign()` and `sign_text()` are removed. Nothing in this repository calls them.
- **BREAKING (nym-sdk, client-core, nym-crypto):** the derivation material option is removed: `with_derivation_material`, `DerivationMaterial`, `ClientKeys::from_master_key`, `ed25519::KeyPair::from_secret` and the unread `index` field of the ed25519 keypair. Nothing in this repository or in nym-vpn-client uses it. A client's root keys are always random.
- **Ecash key moves off the identity key.** A client that buys its own ticketbooks derives its ecash key from the payment mnemonic through one shared helper. The construction is the one `nym-sdk-session` uses today, with a label that names the purpose.
- **BREAKING (nym-bandwidth-fetcher):** `NyxdCredentialFetcher::new` takes a mnemonic and derives the ecash keypair itself. The constructor that took arbitrary seed bytes is removed. `new_with_keypair` stays for callers that supply their own ecash keypair.
- **One-time ecash key change.** nym-sdk, nym-cli and `nym-sdk-session` clients issue new ticketbooks under a different ecash key after the upgrade.
- **Compatibility requirement.** Every ticketbook and every pending deposit that exists before the upgrade MUST remain usable after it. This holds for nym-vpn-client in particular.
- **Not changed:** the gateway protocol, gateway storage, the credential store and the ecash scheme. The keys held in the key store stay as they are.

## Capabilities

### New Capabilities

- `client-gateway-keys`: how a mixnet client derives, stores, refreshes and resets the keys it presents to a gateway; how registrations made before this change and custom registrations are treated; and what key material the SDK exposes to an application.
- `ecash-user-key-derivation`: how a client that buys its own ticketbooks obtains its ecash user key, which inputs the bandwidth fetcher accepts, and the guarantee that ticketbooks and pending deposits issued before a change of key source remain usable.

### Modified Capabilities

- `dvpn-session`: the requirement on mnemonic-funded ticketbook issuance changes. The issuance key is no longer derived by a session-specific hash but through the shared mnemonic helper, which changes the resulting ecash key once.

## Impact

- **common/client-core** (`init`, `client/base_client`, `client/key_manager`): key derivation, the keys type carried by the initialisation result, and the refresh path.
- **common/client-core/gateways-storage**: one sqlite migration for the generation id on a stored registration, and the same field in the in-memory and wasm stores. `GatewaysDetailsStore` gains an operation that replaces a stored registration in place, and every implementation in the repository is updated: on disk, in memory, wasm and the SDK's custom-storage example. The reply store is not changed. After the migration an older binary cannot open the registrations database.
- **common/wasm/client-core**, **wasm/client**, **common/socks5-client-core**, **service-providers/network-requester**: consumers of the client-core initialisation result.
- **sdk/rust/nym-sdk**: the refresh flag, the reset call, removal of `identity_keypair()`, `sign()` and `sign_text()`, and `create_bandwidth_client` no longer reading the client keys.
- **nym-registration-client**: surfaces the refresh flag and the reset, since it builds the mixnet client that nym-vpn-client uses for its main tunnel.
- **common/crypto**: removal of `DerivationMaterial`, `ed25519::KeyPair::from_secret` and the keypair's `index` field. `KeyPair::new` no longer draws a value for that field, so tests elsewhere in the workspace that pin keys from a seeded generator may need new values.
- **common/credentials**, **common/bandwidth-fetcher**: the mnemonic-to-ecash-key helper and the fetcher constructors.
- **common/commands** (nym-cli `ecash issue-ticket-book`, `recover-ticket-book`) and **sdk/rust/nym-sdk-session**: callers of the fetcher.
- **nym-vpn-client** (separate repository): its one fetcher call passes seed bytes today and has to pass its ecash keypair through `new_with_keypair`. The resulting key is identical. It calls none of the removed SDK methods.
- **Statistics:** a network-wide unique-user count obtained by matching gateway-published identity hashes across gateways is inflated, because a client no longer has one identity. A gateway's own count rises only when a client refreshes its keys there.
- **Gateways:** no code change. A derived identity is a new client to a gateway, and each abandoned identity leaves a shared-key and bandwidth row behind unless the client sent forget-me when it disconnected.
