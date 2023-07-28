# Building a Cosmos Blockchain Service with the Rust SDK

This tutorial is for Rust developers wanting to interact with the Rust SDK and take a first step at building a service with which to interact with a blockchain.
The key here is to think of the service as a proxy: it interacts with the blockchain _on the client's behalf_, shielding it from the Validator it interacts with, whilst also being shielded from the client by the mixnet!
