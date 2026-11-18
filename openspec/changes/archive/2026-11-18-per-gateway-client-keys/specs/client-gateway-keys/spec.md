## ADDED Requirements

### Requirement: A new gateway registration presents keys derived for that gateway

For every remote gateway registration created after this change, a mixnet client SHALL present an ed25519 identity keypair derived for that gateway, and SHALL use an x25519 encryption keypair derived for that gateway. It MUST NOT present its root identity public key to that gateway, in the registration handshake or in any later authentication. The client's address at that gateway SHALL be composed of the derived identity public key, the derived encryption public key and the gateway's identity key. Derived keys SHALL be unconditional for a new remote registration: no configuration keeps such a registration on the root keys.

The root keys are the ed25519 identity keypair and the x25519 encryption keypair held by the client's key store. They SHALL stay in the key store as they are, and an upgrade MUST NOT rewrite them. The derived keys MUST NOT be persisted: they SHALL be recomputed whenever a registration is loaded.

The acknowledgement key is not presented to anyone and SHALL remain a single key per client.

#### Scenario: The root identity is not shown to a newly registered gateway
- **WHEN** a client with existing root keys registers with a gateway it has no registration for
- **THEN** the identity public key it sends in the registration handshake differs from its root identity public key, and the identity and encryption keys in its address at that gateway are the derived ones

#### Scenario: Two gateways see unrelated keys
- **WHEN** the same client registers with gateway A and with gateway B
- **THEN** its identity public keys at A and at B differ, and its encryption public keys at A and at B differ

#### Scenario: Derived keys are recomputed, not stored
- **WHEN** a client restarts and loads a registration that uses derived keys
- **THEN** it authenticates with the same identity as before, having recomputed it from the root and the stored generation id, and no derived private key exists in any store

### Requirement: Per-gateway keys are derived with HKDF over BLAKE3 from the root identity key

The derivation SHALL be HKDF with BLAKE3 as the hash. The input key material SHALL be the root ed25519 identity private key, which is its 32-byte seed. The salt SHALL be the label `nym-client-gateway-keys-v1`. The info SHALL be the concatenation of a key-type label, the gateway's 32-byte ed25519 identity public key and the 16-byte generation id of that registration. The key-type label SHALL be `identity` for the identity key and `diffie-hellman` for the x25519 key, which is only ever used for Diffie-Hellman. The output SHALL be 32 bytes.

The identity output SHALL be used as an ed25519 seed, and the other output SHALL be used as the x25519 private key. Both keys SHALL be derived from the root identity key. The root x25519 encryption key MUST NOT be an input to either derivation.

A later change to this construction MUST use a different salt version, so that it cannot produce the keys of this one.

#### Scenario: The derivation is deterministic
- **WHEN** the keys for one root identity key, one gateway identity key and one generation id are derived twice
- **THEN** both derivations yield the same identity keypair and the same encryption keypair

#### Scenario: Every input separates the output
- **WHEN** exactly one of the gateway identity key, the generation id or the key type is changed
- **THEN** the derived key changes

#### Scenario: The construction is pinned
- **WHEN** the keys are derived for a fixed root identity key, gateway identity key and generation id
- **THEN** they equal a known-answer vector recorded with the implementation, so that an unintended change to the hash, the labels or the encoding is detected before it strands existing registrations

### Requirement: Each derived registration carries a 16-byte generation id

Every registration that uses derived keys SHALL carry a 16-byte generation id, stored with the registration. A client SHALL draw the generation id as 16 bytes from the operating system's cryptographic random generator for every new registration with a gateway, the first one included, and again at each refresh, which replaces the stored id. A previous generation id MUST NOT be retained once it has been replaced. The all-zero generation id is used only by a client in LP mode, which stores no registration.

The generation id is not secret. It MUST NOT be sent to the gateway.

#### Scenario: A first registration draws a random generation id
- **WHEN** a client registers with a gateway for the first time
- **THEN** the stored generation id for that registration is 16 bytes newly drawn from the operating system's random generator

#### Scenario: A lost registration store is a fresh start
- **WHEN** a client loses its registration store, keeps its root keys, and registers again with a gateway it was registered with before
- **THEN** it presents an identity derived from a newly drawn generation id, which differs from every identity it presented to that gateway before

### Requirement: Registrations made before this change keep using the root keys

A stored registration that carries no generation id SHALL be treated as using the root keys directly: the client SHALL authenticate to that gateway with its root identity keypair, and its address at that gateway SHALL contain its root identity and root encryption public keys. The store migration SHALL leave every pre-existing registration without a generation id.

Upgrading to a version with this change MUST NOT re-register, refresh or delete an existing registration.

#### Scenario: An existing registration still works after the upgrade
- **WHEN** a client with a registration made by an earlier version starts on a version with this change
- **THEN** it authenticates to that gateway with the same identity and the same shared key, its address is unchanged, and the bandwidth credited to it at that gateway is still available

#### Scenario: Reply SURBs handed out before the upgrade still reach the client
- **WHEN** a recipient uses a reply SURB that the client created for an existing registration before the upgrade
- **THEN** the reply reaches the client, because its address at that gateway did not change

### Requirement: Custom gateway registrations use the root keys

A custom gateway registration, as used by a service provider embedded in a node, SHALL always use the root keys, whether it was made before or after this change.

#### Scenario: An embedded service provider keeps a stable address
- **WHEN** a node starts an embedded service provider on a version with this change, on a new node or an existing one
- **THEN** the provider's address contains its root identity and root encryption public keys

### Requirement: A client in LP mode presents keys derived for its gateway

A mixnet client that connects over the Lewes Protocol SHALL register with the gateway it has chosen under the identity keypair derived for that gateway at the zero generation id, and SHALL use the x25519 keypair derived for that gateway in its address and to decrypt the messages it receives. It MUST NOT present its root identity public key to that gateway. An LP client keeps no stored registration, so it SHALL derive at the zero generation id at every start.

A connect over the Lewes Protocol with the `refresh_gateway_keys` flag set SHALL fail with an error, because the refreshed generation id could not be kept. A full reset SHALL apply to an LP client as to any other client.

Sessions with gateways other than the client's home gateway are not covered by this requirement.

#### Scenario: An LP client does not show its root identity
- **WHEN** a client connects over LP
- **THEN** the identity public key it registers with its gateway differs from its root identity public key, and the identity and encryption keys in its address are the ones derived for that gateway

#### Scenario: An LP client keeps its identity at a gateway across starts
- **WHEN** an LP client with a requested gateway starts twice
- **THEN** it registers with that gateway under the same identity both times

#### Scenario: Two LP gateways see unrelated keys
- **WHEN** an LP client connects to gateway A on one start and to gateway B on another
- **THEN** its identity public keys at A and at B differ, and its encryption public keys at A and at B differ

#### Scenario: A refresh is rejected over LP
- **WHEN** an application sets the refresh flag and connects over LP
- **THEN** the connect fails with an error before any gateway is dialled

### Requirement: Keys at a gateway are refreshed only by an explicit flag on connect

An application SHALL be able to request a key refresh when it connects, through the `refresh_gateway_keys` flag. When the flag is set and a registration exists for the gateway being connected to, the client SHALL draw a new random generation id, perform a new registration handshake with that gateway under the identity derived from it, and replace the stored registration, including its shared key and generation id. The flag SHALL apply to that one connect only. Without the flag the client MUST NOT refresh.

Refreshing a registration that carries no generation id SHALL move it to derived keys under a newly drawn generation id. When no registration exists for that gateway the flag SHALL have no effect beyond the ordinary first registration.

The stored registration MUST be replaced only after the new registration has succeeded. If it fails, the stored registration SHALL be left as it was and the connect SHALL fail with an error: the client MUST NOT fall back to the stored registration for that connect. The replacement SHALL be a single store operation, so that an interruption cannot leave the gateway without a stored registration or with a mix of old and new values.

As part of a refresh the client MUST NOT connect to the gateway as its previous identity.

The flag SHALL be settable on the SDK's mixnet client builder and through `nym-registration-client`. It is not exposed by the command-line clients or by the wasm client. This requirement applies to clients that keep stored registrations; over LP the flag is rejected, as required for LP clients.

#### Scenario: A refresh gives the gateway a new client
- **WHEN** an application connects with the refresh flag to a gateway it has a derived registration for
- **THEN** the client registers under an identity that differs from the previous one, the stored generation id is a new random value, and the previous shared key is no longer stored

#### Scenario: No flag, no refresh
- **WHEN** an application connects without the flag to a gateway it has a registration for
- **THEN** the client authenticates with the stored registration and its identity at that gateway is unchanged

#### Scenario: The flag does not carry over
- **WHEN** an application connects once with the flag and later connects again without it
- **THEN** the second connect authenticates with the registration made by the first and does not refresh

#### Scenario: A legacy registration is refreshed onto derived keys
- **WHEN** an application connects with the flag to a gateway whose stored registration carries no generation id
- **THEN** the client registers under an identity derived from a newly drawn generation id, which differs from its root identity

#### Scenario: A failed refresh leaves the old registration in place
- **WHEN** the registration handshake of a refresh fails
- **THEN** the connect returns an error, and the stored registration, shared key and generation id are unchanged

#### Scenario: The old registration is still usable after a failed refresh
- **WHEN** a refresh has failed and the application connects again without the flag
- **THEN** the client authenticates with the stored registration under its previous identity

#### Scenario: An interrupted refresh leaves a usable registration
- **WHEN** the client stops while a refresh is replacing the stored registration
- **THEN** the store holds either the whole previous registration or the whole new one, and the gateway is still the active gateway

### Requirement: A full reset replaces the root keys and keeps the credential store

The SDK SHALL provide an explicit reset operation, `reset_client_identity`, also reachable through `nym-registration-client`. A reset SHALL generate new random root keys, remove every stored gateway registration and clear the reply store of its reply SURBs and reply keys. A reset MUST NOT modify the credential store, MUST NOT contact any gateway and MUST NOT happen implicitly. It SHALL be available only on a client that is not connected.

A reset replaces the root because an LP client derives its keys from the root at the zero generation id at every start, so removing registrations alone would not give an LP client a fresh identity.

#### Scenario: A reset makes a brand-new client
- **WHEN** an application resets a client and then connects to a gateway it was registered with before
- **THEN** the client presents an identity that gateway has never seen, derived from the new root

#### Scenario: Ticketbooks survive a reset
- **WHEN** an application resets a client whose credential store holds ticketbooks
- **THEN** the credential store is unchanged and those ticketbooks can still be spent

#### Scenario: Nothing from the old address is left in the reply store
- **WHEN** a reset has completed
- **THEN** the reply store holds no reply SURBs and no reply keys

### Requirement: The SDK does not hand the client identity key to applications

The SDK's `MixnetClient` SHALL NOT provide a method that returns the client's identity keypair, and SHALL NOT provide a method that signs application data with it. `identity_keypair()`, `sign()` and `sign_text()` are removed. An application that needs a stable name or a signing key MUST supply its own key material.

Signing with the root identity key is not offered as a replacement, because a verifier needs the root public key and that key is a global identifier.

#### Scenario: An application cannot sign with the client identity
- **WHEN** an application is built against the SDK
- **THEN** `MixnetClient` offers no `identity_keypair`, `sign` or `sign_text`

### Requirement: Root keys are always random

A client's root keys SHALL be generated randomly, at first start and at a reset. The option to derive a client's identity from caller-supplied derivation material (`with_derivation_material`, `DerivationMaterial`) is removed, and neither first-time key generation nor a reset SHALL accept such material.

#### Scenario: A reset yields a root the application cannot predict
- **WHEN** an application resets a client
- **THEN** the new root identity is random and cannot be recomputed from any value the application supplied

### Requirement: The gateway protocol and gateway storage are unchanged

The registration handshake, the authentication request and the data a gateway stores per client SHALL be unchanged by this change. A gateway that has not been upgraded SHALL accept a derived identity as it accepts any new client.

#### Scenario: An upgraded client registers with an older gateway
- **WHEN** a client on a version with this change registers with a gateway running an earlier version
- **THEN** registration, authentication and bandwidth accounting work as they do for any new client

### Requirement: The client statistics id follows the identity in the client's address

The id a client reports in its optional statistics report SHALL keep being derived from the identity public key in its current address. It is therefore distinct per gateway and changes with a refresh. It MUST NOT be derived from the root keys.

#### Scenario: The statistics id changes with the gateway
- **WHEN** a client reports statistics while registered with gateway A and later while registered with gateway B under derived keys
- **THEN** the two reports carry different client ids
