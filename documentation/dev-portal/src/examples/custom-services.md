# Custom Services 
Custom services involve two pieces of code that communicate via the mixnet: a client, and a custom server/service. This custom service will most likely interact with the wider internet / a clearnet service on your behalf, with the mixnet between you and the service, acting as a privacy shield. 

- PasteNym is a private pastebin alternative. It involves a browser-based frontend utilising the Typescript SDK and a Python-based backend service communicating with a standalone Nym Websocket Client. **If you're a Python developer, start here!**.
  - [Frontend codebase](https://github.com/notrustverify/pastenym)
  - [Backend codebase](https://github.com/notrustverify/pastenym-frontend) 
  
- Nostr-Nym is another application written by [NoTrustVerify](https://notrustverify.ch/), standing between mixnet users and a Nostr server in order to protect their metadata from being revealed when gossiping. **Useful for Go and Python developers**.  
  - [Codebase](https://github.com/notrustverify/nostr-nym)
  
- Spook and Nym-Ethtx are both examples of Ethereum transaction broadcasters utilising the mixnet, written in Rust. Since they were written before the release of the Rust SDK, they utilise standalone clients to communicate with the mixnet. 
  - [Spook](https://github.com/EdenBlockVC/spook) (**Typescript**)
  - [Nym-Ethtx](https://github.com/noot/nym-ethtx) (**Rust**)
  
- NymDrive is an early proof of concept application for privacy-enhanced file storage on IPFS. **JS and CSS**, and a good example of packaging as an Electrum app.  
  - [Codebase](https://github.com/saleel/nymdrive)
