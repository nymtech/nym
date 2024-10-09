# Using Your Client

## Proxying traffic
After completing the steps above, your local `nym-socks5-client` will be listening on `localhost:1080` ready to proxy traffic to the Network Requester set as the `--provider` when initialising.

When trying to connect your app, generally the proxy settings are found in `settings->advanced` or `settings->connection`.

Here is an example of setting the proxy connecting in Blockstream Green:

** ![Blockstream Green settings](../../images/blockstream-green.gif)

Most wallets and other applications will work basically the same way: find the network proxy settings, enter the proxy url (host: **localhost**, port: **1080**).

In some other applications, this might be written as **localhost:1080** if there's only one proxy entry field.

## Supported Applications

Any application which can be redirected over Socks5 proxy should work. Nym community has been successfully running over Nym Mixnet these applications:

- Bitcoin Electrum wallet
- Monero wallet (GUI and CLI with monerod)
- Telegram chat
- Element/Matrix chat
- Firo wallet
- Blockstream Green

> DarkFi's ircd chat was previously supported: they have moved to DarkIrc: whether the existing integration work is still operational needs to be tested.

Keep in mind that Nym has been developing a new client **[NymVPN](https://nymvpn.com) (GUI and CLI) routing all users traffic through the Mixnet.**

## Further reading

If you want to dig more into the architecture and use of the socks5 client check out its documentation [here](https://nymtech.net/docs/clients/socks5-client.html).
