# Network Requesters

> The Nym network requester was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code on this page, go there first.


## Current version
```
<!-- cmdrun ../../../../target/release/nym-network-requester --version | grep "Build Version" | cut -b 21-26  -->
```

## Network Requester Whitelist
If you have access to a server, you can run the network requester, which allows Nym users to send outbound requests from their local machine through the mixnet to a server, which then makes the request on their behalf, shielding them (and their metadata) from clearnet, untrusted and unknown infrastructure, such as email or message client servers.

By default the network requester is **not** an open proxy (although it can be used as one). It uses a file called `allowed.list` (located in `~/.nym/service-providers/network-requester/<network-requester-id>/`) as a whitelist for outbound requests.

Any request to a URL which is not on this list will be blocked.

On startup, if this file is not present, the requester will grab the default whitelist from [here](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt) automatically.

This default whitelist is useful for knowing that the majority of network requesters are able to support certain apps 'out of the box'.

**Operators of a network requester are of course free to edit this file and add the URLs of services they wish to support to it!** You can find instructions below on adding your own URLs or IPs to this list.

The domains and IPs on the default whitelist can be broken down by application as follows:

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

# nym matrix server
nymtech.chat

# generic matrix server backends
vector.im
matrix.org

# monero desktop - mainnet
212.83.175.67
212.83.172.165
176.9.0.187
88.198.163.90
95.217.25.101
136.244.105.131
104.238.221.81
66.85.74.134
88.99.173.38
51.79.173.165

# monero desktop - stagenet
162.210.173.150
176.9.0.187
88.99.173.38
51.79.173.165

# alephium 
alephium.org
```

## Network Requester Directory
You can find a list of Network Requesters running the default whitelist [here](https://explorer.nymtech.net/network-components/service-providers). This list comprises of the NRs running as infrastructure for NymConnect.

> We are currently working on a smart-contract based solution more in line with how Mix nodes and Gateways announce themselves to the network.

## Viewing command help

The `./nym-network-requester --help ` command can be used to show a list of available parameters.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nym-network-requester --help -->
```
~~~

You can check the required parameters for available commands by running:

```
./nym-network-requester <command> --help
```

## Initializing and running your network requester

The network-requester needs to be initialized before it can be run. This is required for the embedded nym-client to connect successfully to the mixnet. We want to specify an `id` using the `--id` command and give it a value of your choosing. The following command will achieve that:

```
 ./nym-network-requester init --id example
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun timeout 20s ../../../../target/release/nym-network-requester init --id example -->
```
~~~


Now that we have initialized our network-requester, we can start it with the following command:

```
 ./nym-network-requester run --id example
```

## Upgrading your network requester
You can upgrade your network requester by following these steps:

* stop your network requester service
* replace the old binary with the new binary
* restart your service using the commands in the previous section of the document

### Upgrading to >= v1.1.10 from <v1.1.9

In the previous version of the network-requester, users were required to run a nym-client along side it to function. As of `v1.1.10`, the network-requester now has a nym client embedded into the binary, so it can run standalone.

If you are running an existing network requester registered with nym-connect, upgrading requires you move your old keys over to the new network requester configuration. We suggest following these instructions carefully to ensure a smooth transition.

Initiate the new network requester:

```
nym-network-requester init --id mynetworkrequester
```

Copy the old keys from your client to the network-requester configuration that was created above:

```
cp -vr ~/.nym/clients/myoldclient/data/* ~/.nym/service-providers/network-requester/mynetworkrequester/data
```

Edit the gateway configuration to match what you used on your client. Specifically, edit the configuration file at:

```
~/.nym/service-providers/network-requester/mynetworkrequester/config/config.toml
```

Ensure that the fields `gateway_id`, `gateway_owner`, `gateway_listener` in the new config match those in the old client config at:

```
~/.nym/clients/myoldclient/config/config.toml
```

## Automating your network requester with systemd
Stop the running process with `CTRL-C`, and create a service file for the requester as we did with our client instance previously at `/etc/systemd/system/nym-network-requester.service`:

```ini
[Unit]
Description=Nym Network Requester ({{platform_release_version}})
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=nym # replace this with whatever user you wish
LimitNOFILE=65536
# remember to add the `--enable-statistics` flag if running as part of a service grant and check the path to your nym-network-requester binary
ExecStart=/home/nym/nym-network-requester run --id <your_id>
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target
```

Now enable and start your requester:

```
systemctl enable nym-network-requester.service
systemctl start nym-network-requester.service

# you can always check your requester has succesfully started with:
systemctl status nym-network-requester.service
```

## VPS Setup
### Configure your firewall
Although your requester is now ready to receive traffic, your server may not be - the following commands will allow you to set up a properly configured firewall using `ufw`:

```
# check if you have ufw installed
ufw version
# if it is not installed, install with
sudo apt install ufw -y
# enable ufw
sudo ufw enable
# check the status of the firewall
sudo ufw status
```

Finally open your requester's ssh port to incoming administration connections:

```
sudo ufw allow 22/tcp
# check the status of the firewall
sudo ufw status
```

## Using your network requester

The next thing to do is use your requester, share its address with friends (or whoever you want to help privacy-enhance their app traffic). Is this safe to do? If it was an open proxy, this would be unsafe, because any Nym user could make network requests to any system on the internet.

To make things a bit less stressful for administrators, the Network Requester drops all incoming requests by default. In order for it to make requests, you need to add specific domains to the `allowed.list` file at `$HOME/.nym/service-providers/network-requester/allowed.list`.

### Global vs local allow lists 
Your Network Requester will check for a domain against 2 lists before allowing traffic through for a particular domain or IP. 

* The first list is the default list on the [nymtech.net server](https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt). Your Requester will not check against this list every time, but instead will keep a record of accepted domains in memory. 

* The second is the local `allowed.list` file. 

### Supporting custom domains with your network requester
It is easy to add new domains and services to your network requester - simply find out which endpoints (both URLs and raw IP addresses are supported) you need to whitelist, and then add these endpoints to your `allowed.list`.

> In order to keep things more organised, you can now use comments in the `allow.list` like the example at the top of this page. 

How to go about this? Have a look in your nym-network-requester config directory:

```
ls $HOME/.nym/service-providers/network-requester/

# returns: allowed.list  unknown.list
```

We already know that `allowed.list` is what lets requests go through. All unknown requests are logged to `unknown.list`. If you want to try using a new client type, just start the new application, point it at your local [socks client](../clients/socks5-client.md) (configured to use your remote `nym-network-requester`), and keep copying URLs from `unknown.list` into `allowed.list` (it may take multiple tries until you get all of them, depending on the complexity of the application). Make sure to restart your network requester!

> If you are adding custom domains, please note that whilst they may appear in the logs of your network-requester as something like `api-0.core.keybaseapi.com:443`, you **only need** to include the main domain name, in this instance `keybaseapi.com`

### Running an open proxy
If you *really* want to run an open proxy, perhaps for testing purposes for your own use or among a small group of trusted friends, it is possible to do so. You can disable network checks by passing the flag `--open-proxy` flag when you run it. If you run in this configuration, you do so at your own risk.

## Testing your network requester
1. Add `nymtech.net` to your `allowed.list` (remember to restart your network requester).

2. Ensure that your network-requester is initialized and running.

3. In another terminal window, run the following:

```
curl -x socks5h://localhost:1080 https://nymtech.net/.wellknown/connect/healthcheck.json
```

This command should return the following:

```
{ "status": "ok" }
```


## Ports
### Requester port reference

All network-requester-specific port configuration can be found in `$HOME/.nym/service-providers/network-requester/<YOUR_ID>/config/config.toml`. If you do edit any port configs, remember to restart your client and requester processes.
