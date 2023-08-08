# Interacting with a Cosmos SDK Blockchain via the Mixnet with the Rust SDK

This tutorial is for Rust developers wanting to interact with the Rust SDK and take a first step at building a service with which to interact with a Cosmos SDK blockchain. 

The key here is to think of the service as a proxy: it interacts with the blockchain _on the client's behalf_, shielding the client from the Validator it interacts with, whilst also being shielded from the client by the mixnet.

> This service also nicely highlights the limitations of the mixnet - even though with this code your metadata is shielded from the Validator, and even the service does not know your Nym address, application-level information such as a blockchain address is not made private, in virtue of the fact that using the mixnet provides solely network-level privacy. For information on what application-level privacy Nym offers, check out the [coconut credential SDK example](https://nymtech.net/docs/sdk/rust.html#coconut-credential-generation).
