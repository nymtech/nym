# Overview

There are multiple options to quickly connect to Nym and see the network in action without the need for any code changes to your application. At most, these involve running Nym as a second process alongside an existing application in order to send traffic through the mixnet.  

Demo applications:
* a browser-based 'hello world' [chat application](https://chat-demo.nymtech.net). Either open in two browser windows and send messages to yourself, or share with a friend and send messages to each other through the mixnet!
* a Coconut-scheme based [Credential Library](https://coco-demo.nymtech.net/). This is a WASM implementation of our Coconut libraries which generate raw Coconut credentials. Test it to create and re-randomize your own credentials!
<br>
Proxy traffic with the Nym Socks5 client:
* set up a plug-and-play connection with the [NymConnect](./nymconnect-gui.md) GUI for proxying Telegram, Electrum, Keybase or Blockstream Green traffic through the mixnet (~2 minutes). 
* [Download and run](./socks-proxy.md) the Nym Socks5 client via the CLI, for other desktop applications with SOCKS5 connection options (~30 minutes).

If you've already covered the information in this section, or want to jump straight into integrating/ a Nym connection into an existing application, head to the [Integrations](../integrations/integration-options.md) section. 
