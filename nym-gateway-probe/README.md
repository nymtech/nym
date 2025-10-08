# Nym Gateway Probe

Probe IPv4 and IPv6 interfaces of available gateways to check for the
set that passes a set of minimum service guarantees.

## Build

These instructions assume a debian based system. Adjust accordingly for your
preferred platform.

Install required dependencies

```sh
sudo apt install libdbus-1-dev libmnl-dev libnftnl-dev protobuf-compiler llvm-dev libclang-dev clang
```

Build required libraries and executables

```sh
# build the prober
cargo build -p nym-gateway-probe
```

## Usage

```sh
Usage: nym-gateway-probe [OPTIONS]

Options:
  -c, --config-env-file <CONFIG_ENV_FILE>
          Path pointing to an env file describing the network
  -g, --entry-gateway <ENTRY_GATEWAY>
          The specific gateway specified by ID
  -n, --node <NODE>
          Identity of the node to test
      --min-gateway-mixnet-performance <MIN_GATEWAY_MIXNET_PERFORMANCE>
          
      --min-gateway-vpn-performance <MIN_GATEWAY_VPN_PERFORMANCE>
          
      --only-wireguard
          
  -i, --ignore-egress-epoch-role
          Disable logging during probe
      --no-log
          
  -a, --amnezia-args <AMNEZIA_ARGS>
          Arguments to be appended to the wireguard config enabling amnezia-wg configuration
      
      --netstack-download-timeout-sec <NETSTACK_DOWNLOAD_TIMEOUT_SEC>
          [default: 180]
      --netstack-v4-dns <NETSTACK_V4_DNS>
          [default: 1.1.1.1]
      --netstack-v6-dns <NETSTACK_V6_DNS>
          [default: 2606:4700:4700::1111]
      --netstack-num-ping <NETSTACK_NUM_PING>
          [default: 5]
      --netstack-send-timeout-sec <NETSTACK_SEND_TIMEOUT_SEC>
          [default: 3]
      --netstack-recv-timeout-sec <NETSTACK_RECV_TIMEOUT_SEC>
          [default: 3]
      --netstack-ping-hosts-v4 <NETSTACK_PING_HOSTS_V4>
          [default: nymtech.net]
      --netstack-ping-ips-v4 <NETSTACK_PING_IPS_V4>
          [default: 1.1.1.1]
      --netstack-ping-hosts-v6 <NETSTACK_PING_HOSTS_V6>
          [default: ipv6.google.com]
      --netstack-ping-ips-v6 <NETSTACK_PING_IPS_V6>
          [default: 2001:4860:4860::8888 2606:4700:4700::1111 2620:fe::fe]
  -h, --help
          Print help
  -V, --version
          Print version
```

Examples

```sh
# Run a basic probe against the node with id "qj3GgGYg..."
nym-gateway-probe -g "qj3GgGYgGZZ3HkFrtD1GU9UJ5oNXME9eD2xtmPLqYYw"

# Run a probe against the node with id "qj3GgGYg..." using amnezia with junk packets enabled.
nym-gateway-probe -g "qj3GgGYgGZZ3HkFrtD1GU9UJ5oNXME9eD2xtmPLqYYw" -a "jc=4\njmin=40\njmax=70\n"
```
