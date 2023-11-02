# Key Creation and Use 
The previous example involves ephemeral keys - if we want to create and then maintain a client identity over time, our code becomes a little more complex as we need to create, store, and conditionally load these keys (`examples/builder_with_storage`):

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/builder_with_storage.rs}}
```

As seen in the example above, the `mixnet::MixnetClientBuilder::new()` function handles checking for keys in a storage location, loading them if present, or creating them and storing them if not, making client key management very simple.

Assuming our client config is stored in `/tmp/mixnet-client`, the following files are generated:
```
$ tree /tmp/mixnet-client

mixnet-client
├── ack_key.pem
├── db.sqlite
├── db.sqlite-shm
├── db.sqlite-wal
├── gateway_details.json
├── gateway_shared.pem
├── persistent_reply_store.sqlite
├── private_encryption.pem
├── private_identity.pem
├── public_encryption.pem
└── public_identity.pem

1 directory, 11 files
```
