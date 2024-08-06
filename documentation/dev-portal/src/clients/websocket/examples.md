# Examples
The Nym monorepo includes websocket client example code for Rust, Go, Javacript, and Python, all of which can be found [here](https://github.com/nymtech/nym/tree/master/clients/native/examples).

> Rust users can run the examples with `cargo run --example <rust_file>.rs`, as the examples are not organised in the same way as the other examples, due to already being inside a Cargo project.

All of these code examples will do the following:
* connect to a running websocket client on port `1977`
* format a message to send in either JSON or Binary format. Nym messages have defined JSON formats.
* send the message into the websocket. The native client packages the message into a Sphinx packet and sends it to the mixnet
* wait for confirmation that the message hit the native client
* wait to receive messages from other Nym apps

By varying the message content, you can easily build sophisticated service provider apps. For example, instead of printing the response received from the mixnet, your service provider might take some action on behalf of the user - perhaps initiating a network request, a blockchain transaction, or writing to a local data store.

<!-- THIS PAGE IS NOT WORKING AT THE MOMENT:
> You can find an example of building both frontend and service provider code with the websocket client in the [Simple Service Provider Tutorial](https://nymtech.net/developers/tutorials/simple-service-provider/simple-service-provider.html) in the Developer Portal.
-->
