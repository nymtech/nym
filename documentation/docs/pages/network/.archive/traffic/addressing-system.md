# Addressing System

All clients and nodes in the Nym network have an address, in the format:

```
user-identity-key.user-encryption-key@gateway-identity-key
```

Which in practice, looks something like this:

```
DguTcdkWWtDyUFLvQxRdcA8qZhardhE1ZXy1YCC7Zfmq.Dxreouj5RhQqMb3ZaAxgXFdGkmfbDKwk457FdeHGKmQQ@4kjgWmFU1tcGAZYRZR57yFuVAexjLbJ5M7jvo3X5Hkcf
```

ID keys are used for routing, and encryption keys are the public keypair used to decrypt the exterior layer of Sphinx packets addressed to the node/client.
