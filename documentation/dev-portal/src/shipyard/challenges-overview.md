# Hackathon Challenges
There are a few different challenges to choose from, each with different approaches. It is also recommended to check out the _**Examples**_ directory above for inspiration. 

## Tooling challenge
The tooling challenge involves creating tooling for users, operators, or developers of Nym.

### Examples of user-centric tools:
- Facilitate onboarding new users more easily to staking their Nym, and understanding the pros and cons, as well as finding a good node to stake on. Examples of tools like this:
    - [ExploreNym dashboard](https://explorenym.net/)

- Show information on a dashboard about the network. NOTE due to the amount of dashboards currently available, we expect a good justification for why / something to set this apart from existing ones e.g. it is presenting information that is not already presented, or it is presented in a different manner, such as a TUI or CLI app instead of a web dashboard - maybe an onion service, or no-JS site for those who do not wish to enable Javascript in their day-to-day browsing. Examples of tools like this:
    - [NTV's node dashboard](https://status.notrustverify.ch/d/CW3L7dVVk/nym-mixnet?orgId=1)
    - [IsNymUp dashboard](https://isnymup.com/)

### Examples of operator-centric tooling:
- An APY calculator for determining different financial outcomes of running a node in different situations. 
    
- Scripting for updating and maintaining nodes. Examples of tools like this:
    - [ExploreNym's bash scripts](https://github.com/ExploreNYM/bash-tool) 

- Scripting for packaging node binaries for different OSes.

### Examples of developer-centric tooling:
- Tooling for use in development: are there pain points you’ve found when developing apps with Nym that you have created scripts/hacks/workarounds for? Is there a pain point that you’ve thought ‘oh it would be great if I could just do X’? These are often the best places to start for building out developer tooling - if you’ve run into this issue, it's very likely someone else already has, or will!

- Interacting with one of the SDKs via FFI: perhaps you’re a Go developer who would love to have the functionality of one of the Nym SDKs. Building an FFI tool might be something that would make your life easier, and can be shared with other developers in your situation. 

## Integrations challenge
Integration options for Nym are currently relatively restrictive due to the manner in which Nym handles sending and receiving traffic (as unordered Sphinx packets). This challenge will involve (most likely) implementing custom logic for handling Nym traffic for an existing application.

There are several potential avenues developers can take here:
- If your application (or the application you wish to modify) is written in either Javascript or Typescript, and relies on the `fetch` library to make API calls, then you can use its drop-in replacement: [`mixfetch`](). Perhaps you wish to interact with Coingecko, or a private search engine like Kagi without leaking your IP and metadata, or an RPC endpoint.
    - Example with [NTV’s privacy-preserving Coingecko API](https://github.com/notrustverify/mixfetch-examples)
    - [Mixfetch docs examples](https://github.com/nymtech/nym/tree/develop/sdk/typescript/examples)

- If you instead have an application that is able to use any of the SOCKS5, 4a, or 4 protocols (a rule of thumb: if it can communicate over Tor, it will) then you can experiment with using Nym as the transport layer. 
  - For Rustaceans, check out our [socks5 rust sdk example](https://nymtech.net/docs/sdk/rust.html#socks-client-example).  
  - For those of you who aren’t Crustaceans, then you will have to run the [Socks Client]() alongside your application as a separate process. _NOTE If you are taking this route, please make sure to include detailed documentation on how you expect users to do this, as well as including any process management tools, scripts, and configs (e.g. if you use systemd then include the configuration file for the client, as well as initialisation logic) that may streamline this process._
    - [NTV's PasteNym backend](https://github.com/notrustverify/pastenym) is a great example of an application with this architecture. 

- Nym is not only useful for blockchain-related apps, but for anything that requires network level privacy! Email clients, messaging clients, and decentralised storage are all key elements of the privacy-enabled web. Several of these sorts of apps can be found in the [community apps page](../community-resources/community-applications-and-guides.md). 

- There is currently a proof of concept using Rust Libp2p with Nym as a transport layer. Perhaps you can think of an app that uses Gossipsub for p2p communication could benefit from network-level privacy. 
  - [GossipSub chat example](https://github.com/nymtech/nym/tree/develop/sdk/rust/nym-sdk/examples/libp2p_chat) 
  - [Chainsafe's Lighthouse Nym PoC](https://github.com/ChainSafe/lighthouse/blob/nym/USE_NYM.md#usage)

- Alternatively if you know of an app that is written in Rust or TS and could benefit from using Nym, you could fork and modify it using the SDKs. Applications such as:
  - Magic Wormhole (has a [rust implementation](https://github.com/magic-wormhole/magic-wormhole.rs))
  - [Qual](https://github.com/qaul/qaul.net) (uses Rust Libp2p)
  - [Syncthing](https://github.com/syncthing/syncthing)

### MiniApp challenge 
Write an app, either using one of the SDKs or a standalone client (harder). Think of what you can ‘nymify’ e.g. a version of the [TorBirdy](https://support.torproject.org/glossary/torbirdy/) extension that uses Nym instead of Tor. This is very similar to the Integration challenge in terms of the different potential _architectures_ and approaches, but just for new applications. 
