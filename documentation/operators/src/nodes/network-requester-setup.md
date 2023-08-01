<!---
TODO:
- [ ] Update domains division per app, or preferably maker a version of default allowed list with comments https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt
--->
# Network Requesters

> The Nym gateway was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first.

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

## Current version
```
<!-- cmdrun ../../../../target/release/nym-network-requester --version | grep "Build Version" | cut -b 21-26  -->
```

## Preliminary steps

Make sure you do the preparation listed in the [preliminary steps page](../preliminary-steps.md) before setting up your network requester.

## Network Requester Whitelist
If you have access to a server, you can run the network requester, which allows Nym users to send outbound requests from their local machine through the mixnet to a server, which then makes the request on their behalf, shielding them (and their metadata) from clearnet, untrusted and unknown infrastructure, such as email or message client servers.

By default the network requester is **not** an open proxy (although it can be used as one). It uses a file called `allowed.list` (located in `~/.nym/service-providers/network-requester/<NETWORK-REQUESTER-ID>/`) as a whitelist for outbound requests.

Any request to a URL which is not on this list will be blocked.

On startup, if this file is not present, the requester will grab the default whitelist from [Nym's default list](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) automatically.

This default whitelist is useful for knowing that the majority of network requesters are able to support certain apps 'out of the box'.

**Operators of a network requester are of course free to edit this file and add the URLs of services they wish to support to it!** You can find instructions below on adding your own URLs or IPs to this list.

The domains and IPs on the default whitelist can be broken down by application as follows:

<!---need an update--->
```
# Keybase
keybaseapi.com
s3.amazonaws.com
amazonaws.com
twitter.com
keybase.io
gist.githubusercontent.com

# Used to for uptime healthcheck (see the section on testing your requester below for more)
nymtech.net

# Blockstream Green Bitcoin Wallet
blockstream.info
blockstream.com
greenaddress.it

# Electrum Bitcoin Wallet
electrum.org

# Helios Ethereum Client
alchemy.com
lightclientdata.org
p2pify.com

# Telegram - these IPs have been copied from https://core.telegram.org/resources/cidr.txt as Telegram does
# not seem to route by domain as the other apps on this list do
91.108.56.0/22
91.108.4.0/22
91.108.8.0/22
91.108.16.0/22
91.108.12.0/22
149.154.160.0/20
91.105.192.0/23
91.108.20.0/22
185.76.151.0/24
2001:b28:f23d::/48
2001:b28:f23f::/48
2001:67c:4e8::/48
2001:b28:f23c::/48
2a0a:f280::/32

# Matrix
matrix.org

```

## Network Requester Directory
You can find a list of Network Requesters running the default whitelist in the [explorer](https://explorer.nymtech.net/network-components/service-providers). This list comprises of the NRs running as infrastructure for NymConnect.

> We are currently working on a smart-contract based solution more in line with how Mix nodes and Gateways announce themselves to the network.

## Viewing command help

To begin, move to `/taget/release` directory from which you run the node commands:

```
cd target/release
```

The `./nym-network-requester --help ` command can be used to show a list of available parameters.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-network-requester --help -->
```
~~~

You can check the required parameters for available commands by running:

```
./nym-network-requester <COMMAND> --help
```

> Adding `--no-banner` startup flag will prevent Nym banner being printed even if run in tty environment.

## Initializing and running your network requester

The network-requester needs to be initialized before it can be run. This is required for the embedded nym-client to connect successfully to the mixnet. We want to specify an `<ID>` using the `--id` command and give it a value of your choice. The following command will achieve that:

```
 ./nym-network-requester init --id <YOUR_ID>
```

In the following we used `example`.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun timeout 20s ../../../../target/release/nym-network-requester init --id example -->
```
~~~


Now that we have initialized our network-requester, we can start it with the following command:

```
 ./nym-network-requester run --id <YOUR_ID>
```

## Maintenance

For network requester upgrade (including an upgrade from `<v1.1.9` to `>= v1.1.10`), firewall setup, port configuration, API endpoints, VPS suggestions, automation and more, see the [maintenance page](./maintenance.md).


## Using your network requester

The next thing to do is use your requester, share its address with friends (or whoever you want to help privacy-enhance their app traffic). Is this safe to do? If it was an open proxy, this would be unsafe, because any Nym user could make network requests to any system on the internet.

To make things a bit less stressful for administrators, the Network Requester drops all incoming requests by default. In order for it to make requests, you need to add specific domains to the `allowed.list` file at `$HOME/.nym/service-providers/network-requester/allowed.list`.

### Supporting custom domains with your network requester
It is easy to add new domains and services to your network requester - simply find out which endpoints (both URLs and raw IP addresses are supported) you need to whitelist, and then add these endpoints to your `allowed.list`.

How to go about this? Have a look in your nym-network-requester config directory:

```
ls $HOME/.nym/service-providers/network-requester/

# returns: allowed.list  unknown.list
```

We already know that `allowed.list` is what lets requests go through. All unknown requests are logged to `unknown.list`. If you want to try using a new client type, just start the new application, point it at your local [socks client](https://nymtech.net/docs/clients/socks5-client.html) (configured to use your remote `nym-network-requester`), and keep copying URLs from `unknown.list` into `allowed.list` (it may take multiple tries until you get all of them, depending on the complexity of the application). Make sure to restart your network requester!

> If you are adding custom domains, please note that whilst they may appear in the logs of your network-requester as something like `api-0.core.keybaseapi.com:443`, you **only need** to include the main domain name, in this instance `keybaseapi.com`

### Running an open proxy
If you *really* want to run an open proxy, perhaps for testing purposes for your own use or among a small group of trusted friends, it is possible to do so. You can disable network checks by passing the flag `--open-proxy` flag when you run it. If you run in this configuration, you do so at your own risk.

## Testing your network requester
1. Make sure `nymtech.net` is in your `allowed.list` (remember to restart your network requester).

2. Ensure that your network-requester is initialized and running.

3. In another terminal window, run the following:

```
curl -x socks5h://localhost:1080 https://nymtech.net/.wellknown/connect/healthcheck.json
```

This command should return the following:

```
{ "status": "ok" }
```

