## Context

A mixnet client built on `client-core` holds one `ClientKeys` set (`common/client-core/src/client/key_manager/mod.rs`): an ed25519 identity keypair, an x25519 encryption keypair and an acknowledgement key. It is generated once, when the key store reports no keys, and it is used for every gateway.

- **Registration.** `setup_new_gateway` (`common/client-core/src/init/mod.rs`) registers with the same identity keypair at every gateway. The handshake sends the identity public key in the clear and derives the shared key from ephemeral x25519 keys only; the long-term x25519 key plays no part. Reconnects send the identity public key again with a signature.
- **Storage.** The client keeps one row per gateway in `gateways_registrations.sqlite` holding the shared key. The gateway keys its `shared_keys` row, and through it the client's bandwidth, by the client's identity public key.
- **Address.** A client's address is its identity public key, its encryption public key and its gateway.
- **The x25519 key.** The codebase calls it the encryption key, but it is only ever used for Diffie-Hellman: a sender runs an ephemeral key against it and derives a stream-cipher key from the result, and the client does the same in reverse to read a message.
- **Statistics.** For clients that ask to be counted, a gateway publishes an unsalted SHA-256 of the identity on its metrics endpoint (`unique_active_users_hashes`), and the node status API stores those hashes per gateway per day.
- **Ecash.** The ecash user key is `hash_to_scalar(seed)`. nym-sdk's `create_bandwidth_client` and nym-cli pass the mixnet identity private key as that seed. `nym-sdk-session` passes SHA-256 over a label and the mnemonic entropy. nym-vpn-client passes the private key at the cosmos HD path of its account mnemonic. An issued ticketbook stores its own ecash secret and spends from it.
- **Sender tags.** A client uses one sender tag per recipient for anonymous messages. Tags are not persisted: a client generates fresh ones at every start, so a tag never outlives the address it was created under.
- **Derivation material.** A client can derive its identity from a master key and an index through `with_derivation_material`. Nothing in this repository or in nym-vpn-client uses it.
- **LP mode.** A client built with `with_lp(true)` runs `LpBaseClient` (`common/client-core/src/client/base_client/lp_client.rs`) and keeps no stored registration. At every start it picks the requested gateway, or a random LP gateway if none was requested, builds its address from the root keys, and registers with that gateway over LP under the root identity and root encryption key (`client/lp/control/dialer.rs`). Its dialer is built to open sessions with further gateways under the same identity, but nothing opens a session with any gateway other than the home gateway yet.

Constraints that shaped the design:

- `KeyStore`, `GatewaysDetailsStore` and the reply storage backend are traits with several implementations: on disk, in memory, in the wasm storage and in the SDK's custom-storage example. They are free to change where that makes the code simpler, and every implementation in the repository is then updated with them. An embedding application with its own implementation has to follow.
- nym-vpn-client must keep working, and every ticketbook it already holds must stay spendable after it upgrades its dependency on this repository.
- Gateways cannot be assumed to upgrade together with clients.
- PR #7417 (SQLCipher for client storage) is merged. The registration, reply and credential stores are opened with an optional `nym_sqlcipher::DatabaseKey`, which the SDK keeps on `StoragePaths` and `nym-registration-client` passes in. It added no migration to the registration or reply stores and did not change the store traits.

The full investigation, with line references and the reasoning behind each decision, is recorded outside the repository in the design report this change was written from.

## Goals / Non-Goals

**Goals:**

- A gateway cannot link a client to its registrations at other gateways by the keys it sees.
- A gateway cannot link a client's later sessions to its earlier ones by key, when the application asks for a refresh.
- The ecash user key does not depend on any mixnet client key.
- Existing clients keep working without registering again, and every existing ticketbook and pending deposit stays usable.
- No gateway change.

**Non-Goals:**

- Hiding a client's network address or traffic timing from gateways. A refresh made from the same address at the same moment remains linkable that way.
- Unlinking registrations made before this change. Gateways that already know a client's identity keep knowing it until the client refreshes or resets.
- Carrying bandwidth across a refresh or a reset. Reusing a leftover balance means proving to the gateway that it is the same account, which is the link a refresh removes.
- An automatic refresh policy. The change provides the trigger only.
- A refresh or reset for the wasm client.
- Gateway-side clean-up of abandoned client records.
- Making an ecash blacklist robust. The ecash key in an issuance request is the requester's free choice under any seed scheme.
- Moving nym-vpn-client onto the shared ecash helper. It supplies its own ecash keypair.
- Renaming the existing `encryption_keypair` items, or re-enabling the wasm client's use of the reuse-connection path.
- Per-gateway keys for LP sessions with gateways other than the client's home gateway. Nothing opens such sessions yet, and what they register and how traffic arriving through them is decrypted is not decided.
- A key refresh over LP, which would need a stored generation id that an LP client does not keep.
- The LP dialer's handshake keypair, which is fresh at every start and shared by the sessions of that run.

## Decisions

### Decision 1: Keep the existing keys as a private root and derive per-gateway keys from it

The stored `ClientKeys` stay exactly as they are and act as the root. For a new remote registration the client derives the identity and encryption keys for that gateway and recomputes them whenever the registration is loaded. This is unconditional: there is no switch that keeps a new remote registration on the root keys.

Why: nothing new and secret has to be stored, so the passphrase protection of the key files keeps covering every identity and the key store keeps holding exactly what it holds today.

Alternative considered: random keys per gateway, stored with the registration. Rejected because it puts private keys in a database that is only protected when its encryption is switched on, outside the passphrase protection of the key files.

### Decision 2: Both per-gateway keys come from the root identity key

The x25519 key is derived per gateway too, because it is part of the address and two addresses of one client could otherwise be matched by it. Both derivations use the root ed25519 identity private key as input.

Why: one secret, the root identity key, together with the stored generation ids, determines every per-gateway identity and address.

Consequence: the stored root x25519 key is used only by legacy and custom registrations. New clients still generate and store it, because `ClientKeys` is unchanged, but nothing new uses it.

Alternatives considered: deriving it from the root x25519 key, which gives that key a job but needs two secrets to restore a client; and no longer generating the root x25519 key. Legacy and custom registrations still need it, so it would have to become an optional key in `ClientKeys` and in every key store, which adds states instead of removing code.

### Decision 3: HKDF over BLAKE3 with fixed labels

```
PRK           = HKDF-Extract(salt = "nym-client-gateway-keys-v1", IKM = root identity private key)
id_seed(G, n) = HKDF-Expand(PRK, info = "identity" || G || n, L = 32)
dh_sk(G, n)   = HKDF-Expand(PRK, info = "diffie-hellman" || G || n, L = 32)
```

`G` is the gateway's 32-byte ed25519 identity public key and `n` is the 16-byte generation id. `id_seed` is used as an ed25519 seed and `dh_sk` as the x25519 private key. The hash is BLAKE3, the same type the gateway handshake already uses for its HKDF (`GatewaySharedKeyHkdfAlgorithm`), through the existing `nym_crypto::hkdf::extract_then_expand`.

The second info label is `diffie-hellman` and not `encryption`, because that is the only thing the key is used for. The two info labels differ in length, which is unambiguous because the gateway key and the generation id that follow are fixed-width.

Why: the derivation is hardened. Computing any per-gateway key needs the root private key, so the root public key, which legacy gateways already know, links nothing. The version in the salt lets a later construction coexist without colliding.

Alternatives considered: SHA-512 and SHA-256, both of which the codebase uses for comparable derivations. All three are sound for 32-byte outputs; BLAKE3 was chosen to match the handshake.

The construction is pinned by a known-answer test. Once clients have registered, a change to the hash, a label or the encoding changes their identities and strands their bandwidth.

### Decision 4: Every stored registration draws a random generation id

The generation id (`GenerationId`) is 16 bytes from the OS random generator, drawn for a client's first registration with a gateway and again at each refresh. The previous value is overwritten, not kept. The all-zero id is used only by the LP client, which stores no registration (Decision 16).

Why random from the first registration: a client knows a gateway only through its stored registration. With a fixed first id, a client that lost or deleted its registrations store would register at that id again and show the gateway its first identity, also at a gateway where it had since refreshed, and it could not tell the two cases apart. With a random id, a lost store is a fresh start at every gateway, and no stored registration ever presents an identity the client has moved away from.

Why not a counter: after a lost store, a counter would restart and each later registration would walk back through identities the gateway has already seen. And because old ids are not kept, a leaked root together with the registrations store can recompute only the current generation at each gateway.

Why 16 bytes: the chance of an accidental repeat after `k` registrations and refreshes at one gateway is about `k^2 / 2^129`. Four random bytes would give about 1% after ten thousand. A repeat would only show the gateway an earlier identity again.

Cost: a client that loses its registrations store keeps its root but gets a new identity at every gateway. The bandwidth credited to its old identities is forfeited, messages queued for them are not collected, and its address changes even at a gateway it returns to. Nothing in this repository or in nym-vpn-client loses the registrations store while keeping the key files: nym-vpn-client deletes both together, the wasm client keeps both in one database, and the SDK keeps both in one directory by default.

Alternatives considered: a monotonic counter; and an all-zero id for the first registration with a random id on refresh. The latter let a client that lost its store recover its identity and bandwidth, but only at a gateway it returned to, which in practice means a pinned gateway, and only where it had never refreshed. Where it had refreshed, the same loss showed the gateway the client's first identity again, and that cost fell on exactly the applications that asked for unlinkability.

### Decision 5: The stored registration carries the generation id, and its absence marks a legacy registration

A remote registration gains one nullable 16-byte value. Empty means the registration uses the root keys directly; a value means derived keys at that generation. The sqlite migration leaves every existing row empty. The in-memory and wasm stores carry the same field.

Custom registrations always use the root keys. They are created by a node for its embedded service providers, which talk only to that node and publish a stable address, so there is nothing to unlink.

Why: existing registrations keep their identity, shared key, address and credited bandwidth with no re-registration, and one field serves as both the marker and the id.

### Decision 6: The keys in use with a gateway have their own type

A new type, `GatewayClientKeys`, holds the identity keypair, the x25519 keypair and the acknowledgement key that a client uses with one gateway. `ClientKeys` resolves it from a gateway identity and an optional generation id: derived keys when an id is given, the root keys when none is, and the root acknowledgement key in every case. A stored registration supplies its gateway and its id, which is absent for a legacy or custom registration; the LP client, which stores no registration, supplies its chosen gateway and the all-zero id, `GenerationId::LP`, which no stored registration carries. `InitialisationResult` carries a `GatewayClientKeys`, and the gateway client, the address, the received-message decryption and the statistics id read from it.

`ClientKeys` remains the root type, and the only type a key store loads or saves.

Why a separate type: if the initialisation result carried derived keys in a `ClientKeys`, nothing would stop code from saving them over the root or from deriving again from keys that are already derived. With two types both mistakes fail to compile.

`GatewaySetup::ReuseConnection` and `try_reuse_connection` are kept and carry the new type. They exist so a client can take over the connection made during registration instead of connecting twice. Their only caller, in the wasm client, was commented out when that client's start-up was restructured, and restoring it is not part of this change.

### Decision 7: A refresh is a one-shot flag on connect

The SDK builder gains `refresh_gateway_keys`. The SDK's gateway setup at connect chooses between loading the stored registration for the chosen gateway and registering anew. With the flag set, the load branch instead draws a new generation id, registers under the identity derived from it, and replaces the stored registration, shared key and generation id together, only after the new registration succeeded. `setup_new_gateway` refuses a gateway that already has a registration, so the refresh is its own path.

The replacement is one new operation on `GatewaysDetailsStore`, in the manner of the existing `update_gateway_published_data`, which the on-disk backend performs as a single update. Removing the registration and storing it again with the existing operations was rejected: on disk that is five separate statements with no transaction around them, and removing the active gateway's registration also clears the active-gateway pointer, so an interruption could leave a half-written registration.

- The flag applies to that connect only.
- With no stored registration it changes nothing: the client makes an ordinary first registration.
- A legacy registration moves to derived keys under a newly drawn generation id.
- If the new registration fails, the connect fails. The client is never quietly connected under the identity the application asked to leave. The old registration stays stored for a later connect without the flag.
- The client never connects as its old identity during a refresh. Forget-me for the old identity therefore has to be sent when the previous session disconnects, which the SDK already supports.
- The flag is exposed by the SDK and by `nym-registration-client`, which builds the mixnet client nym-vpn-client uses. The native and socks5 clients offer the same refresh as their `refresh-gateway-keys` command, which refreshes without starting the client; the wasm client does not expose it. Over LP the flag fails the connect (Decision 16).

Why at connect: it is the least linkable moment, since no connection as the old identity precedes it, and the cheapest, since a client whose balance is low would spend a ticket there anyway.

Alternatives considered: refreshing automatically at every connect, which costs one ticket per session, and refreshing whenever a new ticket is needed, which links the sessions that share one ticket. Both stay available to an application through the flag.

### Decision 8: A full reset replaces the root keys

`reset_client_identity` generates new random root keys, removes every gateway registration and clears the reply store, and leaves the credential store alone. It is available on a client that is not connected, it contacts no gateway, and it is exposed by the SDK, by `nym-registration-client` and as the `reset-client-identity` command of the native and socks5 clients. For on-disk storage it works from `StoragePaths`, which already holds the database key the registration and reply stores are opened with. The reply store's files are deleted without opening it, so a store that cannot be loaded cannot fail the reset, and the client's next start creates a fresh one from its own reply SURB configuration. A wrong database key is still caught by the registrations store, which is opened with the same key before anything is removed.

Why replace the root: removing the registrations alone would give a fresh identity at every gateway with a stored registration, since each new registration draws a random generation id, but not to an LP client, which derives at the zero id from the root at every start. A new root covers both and needs no bookkeeping.

Because the ecash key no longer derives from the root (Decision 11), a reset does not change it.

### Decision 9: Sender tags need no change

A client uses one sender tag per recipient, and an earlier draft of this design dropped stored tags whenever the client's address changed. That is not needed. Since #5876 the reply store persists no sender tags (`fs_surbs_migrations/20250702120000_remove_sender_tag.sql`), and a client generates fresh ones at every start. A refresh or a reset happens between runs, so a tag can never span an address change. The reply store and its backend trait are therefore not touched by this change.

### Decision 10: The SDK stops exposing the identity key

`MixnetClient::identity_keypair()`, `sign()` and `sign_text()` are removed. Nothing in this repository calls them; they date from a time when IP packet router requests were meant to carry a client signature, and the current request types use sender tags. nym-vpn-client does not call them either.

Alternatives considered: keeping them on the per-gateway identity, where an application would see its "identity" change with the gateway, and adding root-identity variants, where every signature hands out a global identifier.

### Decision 11: The ecash user key comes from the payment mnemonic through one shared helper

The helper computes `seed = SHA-256("nym-ecash-user-key-from-mnemonic-v1" || mnemonic entropy)` and feeds it to the existing seeded ecash key generation. It takes a `&bip39::Mnemonic`, so a caller cannot pass an unrelated secret. This is `nym-sdk-session`'s construction with a label that names the purpose and not that SDK. The helper lives in `nym-credentials`, the lowest ecash crate that can take a `&bip39::Mnemonic` without a new dependency.

Why the mnemonic: the ecash key belongs to the account that pays, and gateways never see it, so it should not change when network keys do. It is also the only place a seed is needed: a running mixnet client never issues, and the paths that deposit already hold the mnemonic.

Why a new label: one helper serves nym-sdk, nym-cli and `nym-sdk-session`, so the label should not name one of them. The price is that `nym-sdk-session` clients change ecash key once, like nym-sdk and nym-cli clients.

The helper is pinned by a known-answer test, since a changed output changes the ecash key of every self-funding client.

Alternatives considered: nym-vpn-client's formula, the account private key at the cosmos HD path. It is one-way and safe for today's uses, but its only separation is the ecash library's general hash-to-field tag, which the pay-info hash shares, and it hands the raw signing key to the ecash code. Also considered: keeping the ecash keypair in the credential store, and leaving the seed on the identity key, which a reset would then change.

### Decision 12: The fetcher takes a mnemonic or a ready keypair, never seed bytes

`NyxdCredentialFetcher::new` takes a mnemonic and derives the keypair with the helper. `new_with_keypair` stays and uses the keypair it is given unchanged. The constructor that took arbitrary bytes is removed; it is how the identity key became the seed.

nym-vpn-client derives its ecash key next to its account mnemonic and supplies only the keypair, so the mnemonic never has to reach the fetcher. Its current call passes seed bytes, and `new_with_keypair` with the keypair from the same seed yields the identical key.

A recovery-only fetcher never deposits and so needs no ecash key at all. Both fetchers are built over a part that holds no key: the chain client, the pending-deposit store, the signing-data fetchers and the steps that complete a deposit already made. Only `NyxdCredentialFetcher` adds the ecash keypair and the lock that serialises its deposits, so the recovery fetcher holds no keypair, not even a placeholder, and has no way to deposit. Before this change it passed an empty seed. Alternative considered: an optional keypair that a deposit checks, which adds an error branch no public constructor can reach.

Not possible cheaply: deriving from the chain client itself with no extra argument. The fetcher is generic over its client, and the signer trait exposes accounts and signing, not the mnemonic.

### Decision 13: The statistics id keeps following the address identity

The client's optional statistics report keeps identifying it by a hash of the identity in its address. No code changes. The id becomes distinct per gateway and per generation.

Alternative considered: an id derived from the root, which no gateway could compute but which would give the collector a cross-gateway pseudonym.

### Decision 14: Existing ticketbooks are protected by invariants, not by migration

The requirement that every existing ticketbook stays spendable is met by leaving the relevant code alone:

- A spend builds its proof from the `ecash_secret_key` stored in the `IssuedTicketBook`. It reads no client key, no mnemonic and no fetcher state.
- The fetcher's ecash keypair is read in one place, where `make_deposit` attaches it to a new `IssuanceTicketBook`. The deposit transaction itself uses a fresh ed25519 key.
- Recovery completes a pending deposit from its stored record, which carries its own keypair.
- A ticket is bound to a gateway at spend time, not to a client identity.

The implementation therefore has to keep four things true: no change to the credential store schema or the serialised ticketbook; a spend path independent of client keys and mnemonic; `new_with_keypair` using its keypair unchanged; and no automatic deletion, refresh or re-registration of an existing registration.

These are checked from the finished diff, and are to be demonstrated end to end by the QA pass after merge. No new test is added for the ticketbook's serialised form, which this change does not touch.

### Decision 15: The derivation material option is removed

`DerivationMaterial`, `ClientKeys::from_master_key`, the three `with_derivation_material` builder options (the base client's, the LP client's and the SDK's) and `ed25519::KeyPair::from_secret` are removed, together with the `index` field on the ed25519 keypair, which every constructor writes and nothing reads.

Why: nothing in this repository or in nym-vpn-client uses the option, and keeping it would force a decision about what a reset means for a client whose root comes from a master key. With it gone, a root is always random.

Cost: an application outside those two repositories that uses the option breaks at compile time. And `KeyPair::new` draws a random number for the `index` field before it generates the key, so removing the field changes what a seeded test generator produces afterwards. Tests elsewhere in the workspace that pin keys or addresses may need new values.

### Decision 16: An LP client derives the keys for its home gateway at generation zero

After `LpBaseClient` has chosen its gateway, it resolves the `GatewayClientKeys` for that gateway at the zero generation id and uses them for its address, for the registration its dialer sends, and for decrypting the messages it receives. It does so at every start, since it stores no registration.

- `refresh_gateway_keys` set on a connect over LP fails that connect with an error. A refreshed id could not be kept, so the next start would silently return to generation zero.
- A full reset applies to an LP client like any other, because it replaces the root.
- The dialer holds the home gateway's derived keys. A comment on its `request()`, which would open sessions with further gateways and which nothing calls yet, records that each such session needs keys derived for its own gateway first.

Why: an LP client that requests no gateway picks a random one at every start, so under its root identity it shows one identity to a different gateway each run, which is the strongest form of the link this change removes.

Cost: an LP client's identity at its gateway changes once on upgrade, and a client with a requested LP gateway changes address once. Bandwidth an LP gateway credited to the root identity, if any, is not reachable under the derived identity.

Alternatives considered: leaving LP on the root keys as a non-goal, rejected for the reason above; and deriving per dialled gateway, deferred because the meaning of a session with a further gateway is not decided.

## Risks / Trade-offs

- **A leaked root reveals the LP identity at any gateway.** Together with the registrations store it also reveals the current identity at each gateway with a stored registration; earlier generations stay hidden because their ids are not kept. A leaked derived key reveals nothing else.
- **The root identity key has two jobs.** It is the HKDF input and, for legacy and custom registrations, still a signing key. The two go through different functions (HMAC under the protocol label, and ed25519's own hash of the seed). It no longer seeds ecash.
- **A refresh or a reset forfeits the bandwidth credited to the old identity** and makes the client unreachable at its old address. Both happen only when the application asks.
- **A lost registrations store is a fresh start.** A client that loses or deletes it gets a new identity at every gateway, forfeits the bandwidth credited to the old identities and changes address, even at a gateway it returns to (Decision 4).
- **Abandoned identities pile up at gateways.** A gateway removes a client record only on the client's forget-me. Each refresh, reset or lost registrations store leaves a shared-key and bandwidth row behind unless forget-me was sent on the previous disconnect.
- **Network-wide unique-user counts are inflated.** Matching published identity hashes across gateways stops working, and a refreshed client counts more than once at one gateway. This is accepted: a value that de-duplicates a client across gateways is a value that links it.
- **Legacy registrations stay linkable** to the gateways that hold them until refreshed or reset.
- **Two kinds of registration** have to be supported for as long as legacy rows exist.
- **External breakage.** Removing the SDK signing calls, the derivation material option and the seed-bytes constructor breaks external callers at compile time. nym-vpn-client has one affected call site. The new operation on `GatewaysDetailsStore` breaks any implementation of that trait outside this repository in the same way.
- **Pinned test values may change** across the workspace, because of the removed `index` field.
- **One-time ecash key change** for nym-sdk, nym-cli and `nym-sdk-session` clients. Existing ticketbooks are unaffected.
- **No downgrade.** The registrations database is migrated without ignore-missing, so an older binary refuses a database that carries the new migration. This is true of every earlier migration of that database. The wasm store has no such guard: an older wasm build drops the generation id it does not know and treats a derived registration as legacy, so it authenticates with the root identity against a shared key held under the derived one, which fails and shows the gateway the root identity.
- **LP-mode clients change identity once on upgrade**, and one with a requested LP gateway changes address once (Decision 16).
- **Further LP sessions would share the home identity.** If the dialer starts opening sessions with further gateways before per-dial derivation exists, those gateways see the home gateway's derived identity. The comment on `request()` is the only guard.
- **Live wiring is first exercised at QA.** No unit test reaches a real gateway, so a mistake between the derivation and the handshake would be found by the QA pass after merge.
- **The wasm stores were located but not read** during the investigation. The wasm registration is stored as one serde struct with an existing defaulted field, so an added optional field is expected to read old data.

## Migration Plan

1. Land the change as one PR. On first start, a client's registrations database gains the generation id column, empty on every existing row. No other store is migrated.
2. Existing registrations keep authenticating with the root identity. New registrations use derived keys under a random generation id.
3. An application that wants a fresh identity at a gateway sets `refresh_gateway_keys` on connect, ideally with forget-me configured on the preceding disconnect.
4. nym-vpn-client moves its fetcher call to `new_with_keypair` when it upgrades the dependency. No gateway rollout is involved.

Rollback: a client that has run the new version cannot be opened by an older binary without deleting its registrations database, which loses its registrations and the bandwidth credited under them. Its key files and credential store are unaffected and remain readable by the older version.

## QA checklist

These need a real gateway and funded accounts, so they belong to the QA pass against sandbox after merge, not to the implementation tasks.

- **Upgrade from the released version.** A client with an existing registration and existing ticketbooks is upgraded. It connects with that registration without registering again, and spends a ticket from a ticketbook issued before the upgrade.
- **Fresh client.** A new client registers with a gateway, restarts and authenticates again. The identity the gateway sees differs from the client's root identity.
- **Refresh.** A connect with `refresh_gateway_keys` registers under a new identity, and the next connect without the flag keeps that identity.
- **LP mode.** A client connecting with `with_lp(true)` registers with its LP gateway under an identity that differs from its root identity, receives a message sent to its address, and presents the same identity to the same requested gateway on its next start.

## Open Questions

None remain. What a recovery-only fetcher is constructed with is settled in Decision 12.