# Troubleshooting VPS Setup

## Incorrect Gateway Network Check

If you followed all the steps listed in [Connectivity Test and Configuration](../nodes/vps-setup.md#connectivity-test-and-configuration) chapter of VPS Setup and you still have a problem with a correct connectivity for  page in

1. Tor community created a helpful [table of ISPs](https://community.torproject.org/relay/community-resources/good-bad-isps/). Make sure your one is listed there as a *"good ISP"*. If not, consider migrating!
2. Checkout your VPS dashboard and make sure your IPv6-public enabled.
3. If you are able to add IPv6 address `/64` range, do it.

![](../images/ipv6_64.png)


## Virtual IPs and hosting via Google & AWS

For true internet decentralization we encourage operators to use diverse VPS providers instead of the largest companies offering such services. If for some reasons you have already running AWS or Google and want to setup a `<NODE>` there, please read the following.

On some services (AWS, Google, etc) the machine's available bind address is not the same as the public IP address. In this case, bind `--host` to the local machine address returned by `$(curl -4 https://ifconfig.me)`, but that may not the public IP address to bond your `<NODE>` in the wallet.

You can run `ifconfig` command. For example, on a Google machine, you may see the following output:

```sh
ens4: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1460
        inet 10.126.5.7  netmask 255.255.255.255  broadcast 0.0.0.0
        ...
```

The `ens4` interface has the IP `10.126.5.7`. But this isn't the public IP of the machine, it's the IP of the machine on Google's internal network. Google uses virtual routing, so the public IP of this machine is something else, maybe `36.68.243.18`.

To find the right IP configuration, contact your VPS provider for support to find the right public IP and use it to bond your `<NODE>` with the `nym-api` via Nym wallet.

On self-hosted machine it's a bit more tricky. In that case as an operator you must be sure that your ISP allows for public IPv4 and IPv6 and then it may be a bit of playing around to find the right configuration. One way may be to bind your binary with the `--host` flag to local address `127.0.0.1` and run `echo "$(curl -4 https://ifconfig.me)"` to get a public address which you use to bond your Mix Node to `nym-api` via Nym wallet.

It's up to you as a node operator to ensure that your public and private IPs match up properly.
