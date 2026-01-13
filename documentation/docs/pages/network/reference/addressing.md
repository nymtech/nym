# Addressing System

All clients and nodes in the Nym Network have an address that uniquely identifies them for routing purposes.

## Address Format

```
<user-identity-key>.<user-encryption-key>@<gateway-identity-key>
```

### Components

| Component | Purpose |
|-----------|---------|
| `user-identity-key` | Identifies the client for routing |
| `user-encryption-key` | Public key for encrypting messages to this client |
| `gateway-identity-key` | Identifies the client's registered Entry Gateway |

### Example

```
DguTcdkWWtDyUFLvQxRdcA8qZhardhE1ZXy1YCC7Zfmq.Dxreouj5RhQqMb3ZaAxgXFdGkmfbDKwk457FdeHGKmQQ@4kjgWmFU1tcGAZYRZR57yFuVAexjLbJ5M7jvo3X5Hkcf
```

## Key Functions

### Identity Key

- Used by the network to route packets to the correct client
- Derived from the client's Ed25519 keypair
- Base58-encoded for human readability

### Encryption Key

- Used to encrypt the final layer of Sphinx packets
- Only the client can decrypt messages addressed to them
- Derived from the client's X25519 keypair

### Gateway Key

- Identifies which Gateway holds messages for this client
- Clients register with a specific Gateway on connection
- Messages are delivered to this Gateway for pickup

## Address Generation

When a Nym client initializes:

1. Generates Ed25519 keypair (identity)
2. Generates X25519 keypair (encryption)
3. Registers with a Gateway
4. Combines keys and Gateway ID into the address

## Routing Process

When sending to a Nym address:

1. Sender extracts the Gateway key from the address
2. Constructs Sphinx packet with Gateway as final hop
3. Gateway receives packet, identifies recipient by identity key
4. Delivers to recipient (or stores if offline)

## Privacy Considerations

The Nym address does reveal:
- Which Gateway the client uses
- Public keys (but not private keys)

However:
- The Gateway cannot read message contents
- Multiple clients can use the same Gateway
- Addresses can be changed by re-registering

## Address Persistence

By default, clients generate new addresses on each initialization. For persistent identity:
- Store the keypairs securely
- Re-register with the same Gateway
- Address remains constant across sessions
