# Reversed Proxy & Web Secure Socket

It's useful to put your Exit Gateway behind a reversed proxy and have it accessible via `https` domain, where you can host a [landing page](../legal/landing-pages.md). The guide is right [below](#reversed-proxy).

Another solution is to have a your Gateway behind WSS. With ongoing migration from `nym-gateway` to `nym-node --mode exit-gateway` we are working on a detailed guide for WSS setup.

## Reversed Proxy



<!--
## Run Web Secure Socket (WSS) on Gateway

Now you can run WSS on your `nym-node` with an Exit Gateway functionality.

### WSS on a new Gateway

These steps are for an operator who is setting up a [Gateway](gateway-setup.md) for the first time and wants to run it with WSS.

1. Make sure to enable all necessary [ports](maintenance.md#configure-your-firewall) on the Gateway:

```sh
sudo ufw allow 1789,1790,8000,9000,9001,22/tcp, 9001/tcp
```

The Gateway will then be accessible on something like: *http://85.159.211.99:8080/api/v1/swagger/index.html*

Are you seeing something like: *this node attempted to announce an invalid public address: 0.0.0.0.*?

Please modify `[host.public_ips]` section of your config file stored as `~/.nym/gateways/<ID>/config/config.toml`.

### WSS on an existing Gateway

In case you already run a working Gateway and want to add WSS on it, here are the pre-requisites to running WSS on Gateways:

* You need to use the latest `nym-gateway` binary [version](./gateway-setup.md#current-version) and restart it.
* That will add the relevant fields to update your config.
* These two values will be added and need to be amended in your config.toml:

```sh
clients_wss_port = 0
hostname = ""
```

Then you can run this:

```sh
port=$1 // in the example below we will use 9001
host=$2 = // this would be a domain name registered for your Gateway for example: mainnet-gateway2.nymtech.net


sed -i "s/clients_wss_port = 0/clients_wss_port = ${port}/" ${HOME}/.nym/gateways/*/config/config.toml
sed -i "s|hostname = ''|hostname = '${host}'|" ${HOME}/.nym/gateways/*/config/config.toml
```
The following shell script can be run:

```sh
#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: sudo ./install_run_caddy.sh <host_name> <port_to_run_wss>"
    exit 1
fi

host=$1
port_value=$2

apt install -y debian-keyring debian-archive-keyring apt-transport-https
apt --fix-broken install

curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg

curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list

apt update
apt install caddy

systemctl enable caddy.service

cd /etc/caddy

# check if Caddyfile exists, if it does, remove and insert a new one
if [ -f Caddyfile ]; then
    echo "removing caddyfile inserting a new one"
    rm -f Caddyfile
fi

cat  <<EOF >> Caddyfile
${host}:${port_value} {
	@websockets {
		header Connection *Upgrade*
		header Upgrade websocket
	}
	reverse_proxy @websockets localhost:9000
}
EOF

cat Caddyfile

echo "script completed successfully!"

systemctl restart caddy.service
echo "have a nice day!"
exit 0

```

Although your Gateway is Now ready to use its `wss_port`, your server may not be ready - the following commands will allow you to set up a properly configured firewall using `ufw`:

```sh
ufw allow 9001/tcp
```

Lastly don't forget to restart your Gateway, now the API will render the WSS details for this Gateway:

-->
