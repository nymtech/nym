# Nym Echo Server

This is an initial, minimal implementation of an echo server built using the `NymProxyServer` Rust SDK abstraction.

It currently relies on parsing out a `ProxiedMessage` from incoming messages, used by the `NymProxyClient`. In the future it will try and parse a `ReconstructedMessage` type, in order to allow standard `MixnetClient`s to receive echo messages.

You can find the docs [here](https://nym.com/docs/developers/tools/echo-server).
