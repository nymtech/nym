# Anonymous Replies with SURBs (Single Use Reply Blocks)
Both functions used to send messages through the mixnet (`send_message` and `send_plain_message`) send a pre-determined number of SURBs along with their messages by default.

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/src/mixnet/client.rs:33}}
```

You can read more about how SURBs function under the hood [here](https://nymtech.net/docs/architecture/traffic-flow.md#private-replies-using-surbs).

In order to reply to an incoming message using SURBs, you can construct a `recipient` from the `sender_tag` sent along with the message you wish to reply to:

```rust,noplayground
{{#include ../../../../../../sdk/rust/nym-sdk/examples/surb_reply.rs}}
```
