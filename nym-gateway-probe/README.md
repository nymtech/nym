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

The probe supports different test modes via the `--mode` flag:

| Mode | Description |
|------|-------------|
| `mixnet` | Traditional mixnet testing - entry/exit pings + WireGuard via authenticator (default) |
| `single-hop` | LP registration + WireGuard on single gateway (no mixnet) |
| `two-hop` | Entry LP + Exit LP (nested forwarding) + WireGuard tunnel |
| `lp-only` | LP registration only - test handshake, skip WireGuard |

## Usage

### Standard Mode (via nym-api)

Test gateways registered in nym-api directory:

```sh
# Test a specific gateway (mixnet mode)
nym-gateway-probe -g "qj3GgGYgGZZ3HkFrtD1GU9UJ5oNXME9eD2xtmPLqYYw"

# Test with amnezia WireGuard
nym-gateway-probe -g "qj3GgGYg..." -a "jc=4\njmin=40\njmax=70\n"

# WireGuard only (skip entry/exit ping tests)
nym-gateway-probe -g "qj3GgGYg..." --only-wireguard
```

### Localnet Mode (run-local)

Test gateways directly by IP/identity without nym-api:

```sh
# Single-hop: LP registration + WireGuard on one gateway
nym-gateway-probe run-local \
  --entry-gateway-identity "8yGm5h2KgNwrPgRRxjT2DhXQFCnADkHVyE5FYS4LHWLC" \
  --entry-lp-address "192.168.66.6:41264" \
  --mode single-hop \
  --use-mock-ecash

# Two-hop: Entry + Exit LP forwarding + WireGuard
nym-gateway-probe run-local \
  --entry-gateway-identity "$ENTRY_ID" \
  --entry-lp-address "192.168.66.6:41264" \
  --exit-gateway-identity "$EXIT_ID" \
  --exit-lp-address "192.168.66.7:41264" \
  --mode two-hop \
  --use-mock-ecash

# LP-only: Test handshake and registration only
nym-gateway-probe run-local \
  --entry-gateway-identity "$GATEWAY_ID" \
  --entry-lp-address "localhost:41264" \
  --mode lp-only \
  --use-mock-ecash
```

**Note:** `--use-mock-ecash` requires gateways started with `--lp-use-mock-ecash`.

### Split Network Configuration

For docker/container setups where entry and exit are on different networks:

```sh
# Entry reachable from host, exit only reachable from entry's internal network
nym-gateway-probe run-local \
  --entry-gateway-identity "$ENTRY_ID" \
  --entry-lp-address "192.168.66.6:41264" \     # Host → Entry
  --exit-gateway-identity "$EXIT_ID" \
  --exit-lp-address "172.18.0.5:41264" \        # Entry → Exit (internal)
  --mode two-hop \
  --use-mock-ecash
```

## CLI Reference

```
Usage: nym-gateway-probe [OPTIONS] [COMMAND]

Commands:
  run-local  Run probe in localnet mode (direct IP, no nym-api)

Options:
  -c, --config-env-file <PATH>     Path to env file describing the network
  -g, --entry-gateway <ID>         Entry gateway identity (base58)
  -n, --node <ID>                  Node to test (defaults to entry gateway)
      --gateway-ip <IP>            Query gateway directly by IP (skip nym-api)
      --exit-gateway-ip <IP>       Exit gateway IP for two-hop testing
      --mode <MODE>                Test mode: mixnet, single-hop, two-hop, lp-only
      --only-wireguard             Skip ping tests, only test WireGuard
      --only-lp-registration       Test LP registration only (legacy flag)
      --test-lp-wg                 Test LP + WireGuard (legacy flag)
  -a, --amnezia-args <ARGS>        Amnezia WireGuard config arguments
      --no-log                     Disable logging
  -h, --help                       Print help
  -V, --version                    Print version

Localnet Options (run-local):
      --entry-gateway-identity <ID>    Entry gateway Ed25519 identity
      --entry-lp-address <HOST:PORT>   Entry gateway LP listener address
      --exit-gateway-identity <ID>     Exit gateway Ed25519 identity
      --exit-lp-address <HOST:PORT>    Exit gateway LP listener address
      --use-mock-ecash                 Use mock credentials (dev only)
```

## Output

The probe outputs JSON with test results:

```json
{
  "node": "gateway-identity",
  "used_entry": "entry-gateway-identity",
  "outcome": {
    "as_entry": { "can_connect": true, "can_route": true },
    "as_exit": { "can_connect": true, "can_route_ip_v4": true, "can_route_ip_v6": true },
    "wg": { "can_register": true, "can_handshake_v4": true, "can_handshake_v6": true },
    "lp": { "can_connect": true, "can_handshake": true, "can_register": true }
  }
}
```
