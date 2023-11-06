# Using Your Client

The Nym native client exposes a websocket interface that your code connects to. The **default** websocket port is `1977`, you can override that in the client config if you want.

Once you have a websocket connection, interacting with the client involves piping messages down the socket and listening out for incoming messages. 

# Sending Messages 
There are a small number of messages that your application sends up the websocket to interact with the native client, as defined [here](https://github.com/nymtech/nym/blob/develop/clients/native/websocket-requests/src/requests.rs):  

```rust,noplayground
{{#include ../../../../../clients/native/websocket-requests/src/requests.rs:55:97}}
```

## Getting your own address
When you start your app, it is best practice to ask the native client to tell you what your own address is (from the saved configuration files). To do this, send:

```json
{
  "type": "selfAddress"
}
```

You'll receive a response of the format:

```json
{
  "type": "selfAddress",
  "address": "your address" // e.g. "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

## Sending text
If you want to send text information through the mixnet, format a message like this one and poke it into the websocket:

```json
{
  "type": "send",
  "message": "the message",
  "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

In some applications, e.g. where people are chatting with friends who they know, you might want to include unencrypted reply information in the message field. This provides an easy way for the receiving chat to then turn around and send a reply message:

```json
{
  "type": "send",
  "message": {
    "sender": "198427b63ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm",
    "chatMessage": "hi julia!"
  },
  "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
}
```

If that fits your security model, good. However, will probably be the case that you want to send **anonymous replies using Single Use Reply Blocks (SURBs)**.

You can read more about SURBs [here](../../architecture/traffic-flow.md#private-replies-using-surbs) but in short they are ways for the receiver of this message to anonymously reply to you - the sender - without them having to know your nym address.

Your client will send along a number of `replySurbs` to the recipient of the message. These are pre-addressed Sphinx packets that the recipient can write to the payload of (i.e. write response data to), but not view the address. If the recipient is unable to fit the response data into the bucket of SURBs sent to it, it will use a SURB to request more SURBs be sent to it from your client.

```json
{
    "type": "sendAnonymous",
    "message": "something you want to keep secret"
    "recipient": "71od3ZAupdCdxeFNg8sdonqfZTnZZy1E86WYKEjxD4kj@FWYoUrnKuXryysptnCZgUYRTauHq4FnEFu2QGn5LZWbm"
    "replySurbs": 100 // however many reply SURBs to send along with your message
}
```

Each bucket of replySURBs, when received as part of an incoming message, has a unique session identifier, which **only identifies the bucket of pre-addressed packets**. This is necessary to make sure that your app is replying to the correct people with the information meant for them! Constructing a reply with SURBs looks something like this (where `senderTag` was parsed from the incoming message)

```json
{
    "type": "reply",
    "message": "reply you also want to keep secret",
    "senderTag": "the sender tag you parsed from the incoming message"
}
```

## Sending binary data
You can also send bytes instead of JSON. For that you have to send a binary websocket frame containing a binary encoded
Nym [`ClientRequest`](https://github.com/nymtech/nym/blob/develop/clients/native/websocket-requests/src/requests.rs#L25) containing the same information.

As a response the `native-client` will send a `ServerResponse` to be decoded.

You can find examples of sending and receiving binary data in the Rust, Python and Go [code examples](https://github.com/nymtech/nym/tree/master/clients/native/examples), and an example project from the Nym community [BTC-BC](https://github.com/sgeisler/btcbc-rs/): Bitcoin transaction transmission via Nym, a client and service provider written in Rust.

## Error messages
Errors from the app's client, or from the gateway, will be sent down the websocket to your code in the following format:

```json
{
  "type": "error",
  "message": "string message"
}
```

# Listening Out for and Receiving Messages 
Responses to your messages, or if you are running a service that is expecting incoming messages, are defined [here](https://github.com/nymtech/nym/blob/develop/clients/native/websocket-requests/src/responses.rs):

```rust,noplayground
{{#include ../../../../../clients/native/websocket-requests/src/responses.rs:48:53}}
```
