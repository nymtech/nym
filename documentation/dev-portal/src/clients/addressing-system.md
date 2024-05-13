# Addressing System

When a Nym client is initalised, it generates and stores its own public/private keypair locally. When the client starts, it automatically connects to the Nym network and finds out what Nym infrastructure exists. It then chooses and connects to a specific Gateway node via websocket.

All apps in the Nym network therefore have an address, in the format:

```
user-identity-key.user-encryption-key@gateway-identity-key
``` 

Which in practice, looks something like this: 

```
DguTcdkWWtDyUFLvQxRdcA8qZhardhE1ZXy1YCC7Zfmq.Dxreouj5RhQqMb3ZaAxgXFdGkmfbDKwk457FdeHGKmQQ@4kjgWmFU1tcGAZYRZR57yFuVAexjLbJ5M7jvo3X5Hkcf
```

This is obviously not very user-friendly and the moment, and will be developed on in the coming months. 