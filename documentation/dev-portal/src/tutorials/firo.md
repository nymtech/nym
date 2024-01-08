# Firo-Electrum Wallet NymConnect Integration

[Firo](https://github.com/firoorg/firo#firo) (formerly Zcoin) is a privacy focused, zk-proof based cryptocurrency. Now users can enjoy Firo with network privacy by Nym as Firo's fork of Electrum wallet was integrated to work behind the Mixnet. Read more about Firo on their [official webpage](https://firo.org/).

## How can I use Firo over the Nym Mixnet?

> Any syntax in `<>` brackets is a user’s unique variable. Exchange with a corresponding name without the `<>` brackets.

### NymConnect Installation

NymConnect application is for everyone who does not want to install and run `nym-socks5-client`. NymConnect is plug-and-play, fast and easy use. Electrum Bitcoin wallet, Monero wallet (desktop and CLI) and Matrix (Element app) connects through NymConnect automatically to the Mixnet.

1. [Download](https://nymtech.net/download/nymconnect) NymConnect
2. On Linux and Mac, make executable by opening terminal in the same directory and run:

```sh
chmod +x ./nym-connect_<VERSION>
```

3. Start the application
4. Click on `Connect` button to initialise the connection with the Mixnet
5. Anytime you'll need to setup Host and Port in your applications, click on `IP` and `Port` to copy the values to clipboard
6. In case you have problems such as `Gateway Issues`, try to reconnect or restart the application

### Firo Electrum wallet via NymConnect


To download Firo Electrum wallet visit the [Firo's repository](https://github.com/firoorg/firo) or [Github release page](https://github.com/firoorg/electrum-firo/releases/tag/4.1.5.2). To connect to the Mixnet follow these steps:

7. Start and connect [NymConnect](./firo.md#nymconnect-installation) (or [`nym-socks5-client`](https://nymtech.net/docs/clients/socks5-client.html))
8. Start your Firo Electrum wallet
9. Go to: *Tools* -> *Network* -> *Proxy*
10. Set *Use proxy* to ✅, choose `SOCKS5` from the drop-down and add the values from your NymConnect application
11. Now your Firo Electrum wallet runs through the Mixnet and it will be connected only if your NymConnect or `nym-socks5-client` are connected.

![Firo Electrum wallet setup](../images/firo_tutorial/firo.png)
