# ecash-user-key-derivation Specification

## Purpose
Defines how a client that buys its own ticketbooks obtains its ecash user key from the payment mnemonic, which inputs the bandwidth fetcher accepts, and the guarantee that ticketbooks and pending deposits issued before a change of key source remain usable.
## Requirements
### Requirement: A client that buys its own ticketbooks derives its ecash user key from the payment mnemonic

The ecash user keypair that a client in this repository uses for issuing new ticketbooks SHALL be derived from the payment mnemonic by one shared helper. The helper SHALL compute a seed as SHA-256 over a fixed purpose label followed by the mnemonic's entropy, and SHALL turn that seed into the keypair with the existing seeded ecash key generation.

The purpose label SHALL be `nym-ecash-user-key-from-mnemonic-v1`. It is fixed, carries a version, and names the purpose of the derivation and not any one caller. The helper SHALL take the mnemonic as a typed value, not as raw bytes. The mnemonic's raw entropy MUST NOT leave the helper, and the helper SHALL zeroize its copy.

The ecash user key MUST NOT be derived from a mixnet client's identity key or from any other key of a mixnet client.

#### Scenario: One mnemonic gives one ecash key in every caller
- **WHEN** nym-sdk, nym-cli and `nym-sdk-session` each derive the ecash user key from the same mnemonic
- **THEN** all three obtain the same ecash public key

#### Scenario: The ecash key does not depend on the mixnet client
- **WHEN** two mixnet clients with different identity keys buy ticketbooks with the same mnemonic
- **THEN** both issue under the same ecash public key

#### Scenario: A full reset of the mixnet client keeps the ecash key
- **WHEN** a mixnet client's root keys are replaced by a full reset and it then buys a ticketbook with the same mnemonic as before
- **THEN** the ticketbook is issued under the same ecash public key as before the reset

#### Scenario: The derivation is pinned
- **WHEN** the helper is run on a fixed mnemonic
- **THEN** the resulting ecash public key equals a known-answer vector recorded with the implementation, so that an unintended change of the label or the construction is detected

### Requirement: The bandwidth fetcher accepts a mnemonic or a ready ecash keypair

`NyxdCredentialFetcher` SHALL be constructible in exactly two ways with respect to the ecash user key: from a mnemonic, in which case it derives the keypair with the shared helper, or from a caller-supplied ecash keypair, which it SHALL use unchanged. It MUST NOT offer a constructor that takes arbitrary seed bytes.

A fetcher that only recovers pending deposits and never makes a new one SHALL NOT require an ecash user key.

#### Scenario: A caller that derives the key itself keeps its key
- **WHEN** a caller that previously passed seed bytes constructs the fetcher with the keypair it obtains from the same seed
- **THEN** new ticketbooks are issued under the same ecash public key as before

#### Scenario: A caller that holds the mnemonic passes the mnemonic
- **WHEN** a caller constructs the fetcher from a mnemonic
- **THEN** new ticketbooks are issued under the ecash public key the shared helper derives from that mnemonic

#### Scenario: Recovery needs no ecash key
- **WHEN** a recovery-only fetcher completes a pending deposit
- **THEN** it uses the keypair stored with that pending deposit and is constructed without any ecash user key

### Requirement: Self-funding callers do not read mixnet client keys for issuance

The SDK's `create_bandwidth_client` SHALL derive the ecash user key from the mnemonic it is given and MUST NOT read the mixnet client's keys. It SHALL therefore work before the client's keys have been generated.

nym-cli's ticketbook issuance for a client configuration SHALL derive the ecash user key from the mnemonic that pays for the deposit and MUST NOT read the client's identity key file. nym-cli's issuance to a file with an explicitly supplied secret SHALL keep deriving the ecash user key from that secret.

#### Scenario: Bandwidth can be bought before the client keys exist
- **WHEN** an application calls `create_bandwidth_client` on a client with on-disk storage whose keys have not been generated yet
- **THEN** the call succeeds and does not fail on missing client keys

#### Scenario: nym-cli issues for a client under the mnemonic-derived key
- **WHEN** nym-cli issues a ticketbook for a client configuration
- **THEN** the ticketbook is issued under the ecash public key the shared helper derives from the paying mnemonic, and the client's identity key file is not read

#### Scenario: An explicit secret still determines the key
- **WHEN** nym-cli issues a ticketbook to a file with the same explicit secret before and after this change
- **THEN** both ticketbooks are issued under the same ecash public key

### Requirement: Ticketbooks and pending deposits issued before a change of key source remain usable

Every ticketbook held in a credential store written by an earlier version SHALL deserialise and SHALL be spendable after an upgrade to a version with this change. This holds for every client, and for nym-vpn-client in particular once it upgrades its dependency on this repository.

Spending a ticket MUST use only the ecash secret stored in its ticketbook and the published signing data. It MUST NOT require the mnemonic, a mixnet client key or the bandwidth fetcher. This change MUST NOT alter the credential store schema or the serialised form of an issued ticketbook, and an upgrade MUST NOT delete, invalidate or re-issue a stored ticketbook. A partially spent ticketbook SHALL keep its count of spent tickets.

A pending deposit recorded by an earlier version that has not expired SHALL be completable after the upgrade with the keypair stored in its pending record, whatever key the fetcher is constructed with.

A change of key source SHALL affect only ticketbooks issued after it. Ticketbooks issued under different ecash keys SHALL be able to sit in one credential store and SHALL all be spendable.

#### Scenario: A ticketbook issued under the identity-derived key is spent after the upgrade
- **WHEN** a client holds a ticketbook that was issued while the ecash key was seeded from its mixnet identity key, and upgrades
- **THEN** it spends tickets from that ticketbook successfully

#### Scenario: nym-vpn-client's ticketbooks are spent after it upgrades
- **WHEN** nym-vpn-client, whose ticketbooks were issued under the ecash key it derives from its account mnemonic, upgrades its dependency and constructs the fetcher with that same keypair
- **THEN** every ticketbook it already holds can be spent, and its new ticketbooks are issued under the same ecash public key as its old ones

#### Scenario: A deposit pending at upgrade time is completed
- **WHEN** a deposit was made and recorded as pending by an earlier version, and the client upgrades before issuance finished
- **THEN** recovery completes the ticketbook with the keypair stored in the pending record

#### Scenario: Ticketbooks under two ecash keys coexist
- **WHEN** a credential store holds one ticketbook issued before the change of key source and one issued after it
- **THEN** tickets from both can be spent

#### Scenario: A partially spent ticketbook keeps its position
- **WHEN** a client upgrades while holding a ticketbook from which some tickets have been spent
- **THEN** its next spend continues from the stored count and does not reuse a spent ticket

