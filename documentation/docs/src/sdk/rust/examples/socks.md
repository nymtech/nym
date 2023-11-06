# Socks Proxy 
There is also the option to embed the [`socks5-client`](../../../clients/socks5-client.md) into your app code (`examples/socks5.rs`):

```admonish info
If you are looking at implementing Nym as a transport layer for a crypto wallet or desktop app, this is probably the best place to start if they can speak SOCKS5, 4a, or 4.
```

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/socks5.rs}}
```
