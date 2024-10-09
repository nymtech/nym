# Message Types

There are several functions used to send outgoing messages through the Mixnet, each with a different level of customisation:

- `send(&self, message: InputMessage) -> Result<()>`
Sends a `InputMessage` to the mixnet. This is the most low-level sending function, for full customization. Called by `send_message()`.

- `send_message<M>(&self, address: Recipient, message: M, surbs: IncludedSurbs) -> Result<()>`
Sends bytes to the supplied Nym address. There is the option to specify the number of reply-SURBs to include. Called by `send_plain_message()`.

- `send_plain_message<M>(&self, address: Recipient, message: M) -> Result<()>`
Sends data to the supplied Nym address with the default surb behaviour.

> Note we specify *outgoing* messages above: this is because the SDK assumes that replies will be anonymous via [SURBs]() TODO LINK.

Replies rely on the creation of an `AnonymousSenderTag` by parsing and storing the `sender_tag` from incoming messages, and using this to reply, instead of the `Receipient` type used by the functions outlined above:

`send_reply<M>(&self, recipient_tag: AnonymousSenderTag, message: M) -> Result<()>` will send the reply message to the supplied anonymous recipient.

> You can find all of the function definitions [here](https://github.com/nymtech/nym/blob/master/sdk/rust/nym-sdk/src/mixnet/traits.rs).
