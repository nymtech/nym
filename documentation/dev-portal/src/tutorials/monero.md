# Monero NymConnect Integration

*New Nym mixnet integration launched for Monero desktop to secure the right to financial privacy and censorship-resistance*

![](../images/monero_tutorial/monero.png)

Financial privacy is an important component of digital currencies and the use of Nym will provide users with the highest level of privacy at the infrastructure level. All users of digital currencies should be afforded equal rights to protection from financial surveillance.

A team made up of Monero community members have successfully set up a service provider to use Monero (using the Monero desktop wallet) over the Nym mixnet. This allows Monero users to easily use NymConnect to run Monero over the mixnet, thereby enhancing the privacy of Monero transactions.

## How can I use Monero over the Nym mixnet?

The mainnet service provider to Monero over the Nym mixnet is now ready for use via [NymConnect](https://nymtech.net/download-nymconnect/).

* Download and open the latest version of [NymConnect](https://nymtech.net/download-nymconnect/).
* Click on the top left options and go to Settings
* Go to “Select service provider” and turn it on
* For Mainnet, search for this provider or insert it manually:

```sh
i1TiuoNp4jp9weffCW7tPnkb4hRTPydRjX8iXFVaYDG.88Z1hruuvbzWpdCE2xYnTbPNrr49j4s7mmUQC5wvRRLZ@3EPuxwGn2WP2HdxybzoDa5QsohYSP76aQQRUJuPMvk23
```

* Go to the main NymConnect interface and connect to the mixnet

Then go to your Monero wallet (gui or otherwise) and change the settings to run over socks5 proxy:

**Monero desktop:**

* Settings -> Interface -> Socks5 proxy -> Add values: IP address `localhost`, Port `1080`

<iframe width="700" height="400" src="https://www.youtube.com/embed/oSHnk1BG_f0" title="Demo: Connect Your Monero Wallet to the Nym Mixnet via NymConnect" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" allowfullscreen></iframe>

**CLI**

* **Monerod:** add `--proxy 127.0.0.1:1080 --bootstrap-daemon-proxy 127.0.0.1:1080` to args

* **Monero-wallet-{rpc, cli}:** add `--proxy 127.0.0.1:1080 --daemon-ssl-allow-any-cert` to args

Follow the instructions and the Monero mainnet will be connected through to the Nym mixnet.

For those who want to try it out in testnet, a stagenet service provider is also available: [https://nymtech.net/.wellknown/connect/service-providers.json](https://nymtech.net/.wellknown/connect/service-providers.json)
