# Addressing

All clients and nodes in the Nym Network have an address that uniquely identifies them for routing.

## Address format

A Nym address has three parts separated by dots and an @ symbol:

```
<user-identity-key>.<user-encryption-key>@<gateway-identity-key>
```

The **identity key** identifies the client for routing purposes. It's derived from the client's Ed25519 keypair and base58-encoded for readability.

The **encryption key** is the public key used to encrypt the final layer of Sphinx packets destined for this client. Only the client holding the corresponding private key can decrypt messages addressed to them.

The **gateway key** identifies which Gateway holds messages for this client. When you connect, your client registers with a specific Entry Gateway, and that Gateway's identity becomes part of your address.

## Example

```
DguTcdkWWtDyUFLvQxRdcA8qZhardhE1ZXy1YCC7Zfmq.Dxreouj5RhQqMb3ZaAxgXFdGkmfbDKwk457FdeHGKmQQ@4kjgWmFU1tcGAZYRZR57yFuVAexjLbJ5M7jvo3X5Hkcf
```

## How routing works

When sending to a Nym address, the sender extracts the Gateway key and constructs a Sphinx packet with that Gateway as the final hop. The Gateway receives the packet, identifies the recipient by their identity key, and delivers the message (or stores it if the recipient is offline).

Address types: [`common/nymsphinx/addressing`](https://github.com/nymtech/nym/tree/develop/common/nymsphinx/addressing)

## Privacy considerations

The address reveals which Gateway you use and your public keys. It doesn't reveal your IP address or private keys. Multiple clients can use the same Gateway, so the Gateway key alone doesn't identify you.

For persistent identity across sessions, store your keypairs and re-register with the same Gateway. For ephemeral identity, generate new keys each session.
