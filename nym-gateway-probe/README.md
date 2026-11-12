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
cargo build -p nym-gateway-probe
```

## Test Modes

Select phases with `--test-mode` (default: `core`). NS-Agents force `all`.

| Mode | Description |
|------|-------------|
| `core` | Mixnet entry/exit pings + WireGuard via authenticator (default) |
| `wg-mix` | WireGuard via authenticator only |
| `wg-lp` | Entry LP + Exit LP (nested forwarding) + WireGuard |
| `lp-only` | LP registration only (no WireGuard) |
| `socks5-only` | SOCKS5 network-requester HTTPS probe |
| `bridges-only` | Published bridge transports via WG forward path (needs mixnet + credentials) |
| `all` | Mixnet, WireGuard via authenticator, bridges, LP, and SOCKS5 |

Bridge results are informational: they are not used by the directory visual score.
`bridges-only` / `all` register a WG peer so the probe can prove traffic through
each transport, not only that the dial connects. Use a real mnemonic for
authenticator registration; `--use-mock-ecash` is for LP mock gateways and is
not sufficient for the authenticator path.

## Usage

### Bonded gateway (`run`)

```sh
# Default core mode (mixnet + WG)
./nym-gateway-probe -c path/to/network.env run \
  -g "qj3GgGYgGZZ3HkFrtD1GU9UJ5oNXME9eD2xtmPLqYYw" \
  --mnemonic "$TEST_MNEMONIC"

# Same surface as NS-Agents
./nym-gateway-probe -c path/to/network.env run \
  -g "qj3GgGYg..." \
  --test-mode all \
  --mnemonic "$TEST_MNEMONIC"

# Bridges only
./nym-gateway-probe -c path/to/network.env run \
  -g "qj3GgGYg..." \
  --test-mode bridges-only \
  --mnemonic "$TEST_MNEMONIC"

# Amnezia WireGuard args (appended to the WG config)
./nym-gateway-probe -c path/to/network.env run \
  -g "qj3GgGYg..." \
  --test-mode wg-mix \
  --mnemonic "$TEST_MNEMONIC" \
  --amnezia-args "jc=4
jmin=40
jmax=70
"
```

### Local / unannounced (`run-local`)

Probe by IP without directory lookup. Mixnet and authenticator-WG phases that
need topology are limited; LP can still run when the node exposes it.

```sh
./nym-gateway-probe -c path/to/network.env run-local \
  --entry-gateway-ip "192.168.66.6" \
  --test-mode lp-only \
  --use-mock-ecash
```

**Note:** `--use-mock-ecash` requires gateways started with `--lp-use-mock-ecash`.

## Output

The probe prints JSON to stdout. With `--test-mode all`, `outcome` may include
`as_entry`, `as_exit`, `wg`, `bridges`, `lp`, and `socks5` (absent phases are
`null`). Example bridge shape:

```json
{
  "node": "gateway-identity",
  "used_entry": "entry-gateway-identity",
  "outcome": {
    "as_entry": { "can_connect": true, "can_route": true },
    "wg": { "can_register": true, "can_handshake_v4": true, "can_handshake_v6": true },
    "bridges": {
      "transports": [
        {
          "transport": "quic_plain",
          "can_connect_v4": true,
          "can_handshake_v4": true,
          "ping_ips_performance_v4": 1.0,
          "error": null
        }
      ],
      "error": null
    },
    "lp": { "can_connect": true, "can_handshake": true, "can_register": true }
  }
}
```
