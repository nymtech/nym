# Nym Gateway Probe (netstack)

The gateway probe uses `netstack` to do various parts of the gateway test:

- send ICMP pings
- download files
- check the top-up metadata endpoint

## Running locally

You will need:
- a Wireguard config
- be registered with a gateway
- have topped up your bandwidth with the gateway
- a mnemonic for an account with NYM tokens to issue `zk-nyms`

You can get you Wireguard config by running the `nym-gateway-probe` locally:

```
SHOW_WG_CONFIG=true nym-gateway-probe -g ${IDENTITY KEY} run-local --mnemonic=${MNEMONIC}
```

In the probe logs you will see the Wireguard config:

```
private_key=...
listen_port=50239
public_key=...
preshared_key=0000000000000000000000000000000000000000000000000000000000000000
protocol_version=1
endpoint=13.245.9.123:51822
last_handshake_time_sec=0
last_handshake_time_nsec=0
tx_bytes=0
rx_bytes=0
persistent_keepalive_interval=0
```

In the `main()` function, uncomment the lines and set your Wireguard config:

```go
func main() {
	var _, err = ping(NetstackRequestGo{
		WgIp:             "10.1.155.153",
		PrivateKey:       "...",
		PublicKey:        "...",
		Endpoint:         "13.245.9.123:51822",
		MetadataEndpoint: "http://10.1.0.1:51830",
		Dns:              "1.1.1.1",
		IpVersion:        4,
		//PingHosts:          nil,
		//PingIps:            nil,
		//NumPing:            0,
		//SendTimeoutSec:     0,
		//RecvTimeoutSec:     0,
		//DownloadTimeoutSec: 0,
		MetadataTimeoutSec: 5,
		//AwgArgs:            "",
	})

	if err != nil {
		log.Fatal(err)
	}
}
```