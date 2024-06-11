# Tutorial Overview

This tutorial involves writing two pieces of code in Rust:

- A client side binary used to construct a blockchain query and send this query to a service, which will query the Cosmos SDK blockchain and then pass the response back to the client (bear in mind this principle works for all blockchains - we're just utilising the `cosmrs` library to interact with the Sandbox testnet blockchain in this tutorial). This query will be to query the balance of an account, in preparation for spending these tokens on a [bandwidth credential](https://nymtech.net/docs/bandwidth-credentials.html) in a subsequent tutorial.
- A service which will listen out for requests from the mixnet, act on those requests, and anonymously reply to the client sending the requests.

You will learn how to do the following with the Rust SDK:
- Create clients with manual storage settings.
- Parse incoming traffic from the mixnet and reply anonymously using [SURBs](https://nymtech.net/docs/architecture/traffic-flow.html#private-replies-using-surbs).

> Services usually run on remote servers to assure reliable uptime and to unlink sender and receiver metadata. For demonstration purposes however, you will run both components on your local machine, looping messages through the mixnet to yourself.

You can find the code for these components [here](https://github.com/nymtech/developer-tutorials). You can use it as a reference while building or simply download it and follow along as you progress through the tutorial.

Notice that this tutorial attempts to use very few external libraries. This tutorial is not showing you how to build production-grade code, but **to understand how to connect and send messages to, as well as receive messages from, the mixnet.**

```admonish note title="Sidenote: What is a Service / Service Provider?"
'Service' or 'Service Provider' are catchall names used to refer to any type of app that can communicate with the mixnet via a Nym client - in this case, one embedded in its app process via the Rust SDK.

The first SP to have been released is the [Network Requester](https://nymtech.net/docs/nodes/network-requester-setup.html) - a binary that receives a network request from the mixnet, performs that request (e.g. authenticating with a message server and receiving new messages for a user) and then passes the response back to the user who requested it anonymously, shielding their metadata from the message server.

The SP you will build in this tutorial is far more simple than this, showing you how to approach building something that can:
* connect to the mixnet,
* listen for messages, and
* perform some action with them - in this case, query a Cosmos SDK blockchain.

However, once you see how easy it is to integrate with the mixnet for traffic transport, you will be able to build apps with real-world uses easily.
```
