# Free-tier gateway harness (local, standalone)

An integration harness that runs a real `nym-node` gateway in a container with the free tier enabled, so free-tier runtime wiring can be tested end-to-end as it lands. This is the container layer of task 7.3; the bare-kernel datapath tests live in `../netns`.

**Milestone A (this scaffold):** boot an unbonded, standalone entry gateway with the free tier enabled and confirm it comes up. **Milestone B (next):** drive `LpRegistrationClient` + `smol-dvpn`'s explicit `PeerConfig` against it with a minted free-tier JWT, and assert the peer registers, gets pooled, exhausts into the garden, and is released on upgrade.

## Why this is not the production image

`nym-node/Dockerfile` is unsuitable here: it pulls its base images from `harbor.nymte.ch` (Nym VPN only), downloads `nym-cli`, and its entrypoint runs the full mainnet bonding flow. This harness drops all three. It builds `nym-node` from source against Docker Hub bases and runs with `--standalone`, so the node makes no nym-api / nyx-chain / nym-node calls and needs no network egress. It is never bonded, funded, or placed in topology; a client reaches it directly over LP (Milestone B), bypassing topology selection.

`entry-gateway` mode is used (not exit) so the node does not fetch the upstream exit policy - the only external call `--standalone` does not already suppress.

## Files

| File | Purpose |
| --- | --- |
| `Dockerfile.localtest` | Two-stage build of `nym-node` + the datapath runtime deps (`nftables`, `iproute2`, `wireguard-tools`). |
| `entrypoint.localtest.sh` | Single-command boot: `nym-node run` (inits config on first boot, then runs). No init/deny split, no bonding. |
| `run_docker.sh` | Build + run under Docker Desktop, publishing the ports below to `localhost`. |
| `run_apple_container.sh` | Build + run under Apple's `container` runtime (vmnet: a directly host-reachable IP, no gVisor). |
| `deploy_vps.sh` | Cross-compile `nym-node`, ship it to a Linux VPS, and (re)start it under systemd - the only path that can forward WG traffic to the open internet. |
| `.env` | All node + free-tier knobs, including the TEST-ONLY signer keypair. |

## Running

Two runtimes, same image + entrypoint. Both run the register → tunnel → free-tier-credential flow and in-tunnel reachability (e.g. the gateway metadata endpoint). **Neither can forward WG traffic to the open internet** - so the internet-egress / walled-garden *datapath* test must run on a real Linux kernel (see `../netns`, or a Linux CI host with kernel WireGuard):

- **Docker Desktop** (`./run_docker.sh`) - simplest. Has conntrack + forwarding, but its gVisor userspace netstack proxies the connection (the gateway sees loopback-sized MSS from the "internet"), so forwarded replies never complete the handshake at the client.
- **Apple `container`** (`./run_apple_container.sh`) - each container is a micro-VM with vmnet (directly host-reachable IP). But its minimal kernel has **no netfilter conntrack module**, so stateful MASQUERADE/NAT can't work - no internet forwarding at all. Requires the [`container`](https://github.com/apple/container) CLI, needs `-m 2G` (the VM defaults below the node's memory requirement), and builds via `docker save`→`container image load` (`container build` stalls on the large context).

Both need `CAP_NET_ADMIN` (nft/tc + WireGuard). The Docker path also passes `--device /dev/net/tun`; under Apple `container` the micro-VM provides it natively.

```bash
./run_docker.sh              # Docker: gateway on localhost
./run_apple_container.sh     # Apple container: prints the vmnet IP + the driver command
./deploy_vps.sh user 1.2.3.4 # Linux VPS: cross-compile, scp, systemd (re)start
```

### Linux VPS (real kernel)

`deploy_vps.sh <user> <ip>` cross-compiles `nym-node` (`cross build --target x86_64-unknown-linux-gnu`), copies the binary over, and installs/restarts it under a systemd unit launched from the same `.env` (shipped on every deployment). First run installs the unit; later runs just replace the binary and restart. It assumes NAT/forwarding (`network-tunnel-manager.sh apply_iptables_rules_wg`) is already applied on the box. Because the box has a real kernel (conntrack, real NAT, kernel WireGuard available), this is the only harness path that can forward tunnel traffic to the open internet. The node advertises the `ip` argument (override with `NYMNODE_PUBLIC_IPS`). Other knobs: `REMOTE_DIR`, `SERVICE`, `SSH_PORT`, `SKIP_BUILD=1`.

The first build compiles `nym-node` and is slow; later builds reuse the cargo cache mount. Once up, confirm liveness against the HTTP API (default `:8080`), e.g. `GET /api/v1/build-information`, or open the swagger UI it serves.

Under Docker, `run_docker.sh` publishes the ports a client needs (each overridable via env: `HTTP_PORT`, `WG_PORT`, `LP_CONTROL_PORT`, `LP_DATA_PORT`). Under Apple `container` there is no port publishing - the driver connects to the container's vmnet IP directly (`--gateway-http http://<ip>:8080 --gateway-ip <ip>`):

| Port | Proto | Purpose |
| --- | --- | --- |
| 8080 | tcp | node HTTP API (boot health check) |
| 51822 | udp | WireGuard tunnel - the client data plane |
| 41264 | tcp | LP control - the registration transport (Milestone B) |
| 51264 | tcp | LP data channel |

The WireGuard private-metadata endpoint is served in-tunnel, not on a separate host port, so it needs no mapping.

## Demo: the free-tier lifecycle (trial -> throttle -> walled garden)

Once the runtime wiring is deployed to a real Linux gateway (`deploy_vps.sh`), the driver can demonstrate the whole lifecycle client-side. The driver mints a `NewUser` free-tier capability token from the signer key (`--free-tier`), and `--download <url>` bulk-downloads a file through the tunnel (TLS + redirects handled), timed, reporting throughput - or `BLOCKED` if the garden drops it mid-transfer.

Gateway config for the demo - three hidden `nym-node` knobs (hidden from `--help`; set here via `.env`, no config-file edit or rebuild):
- `NYMNODE_FREE_TIER_WALLED_GARDEN_WHITELIST` = the IP of ONE file endpoint (the "purchase endpoint" stand-in), e.g. OVH `141.95.207.211` (comma-separate for several).
- `NYMNODE_FREE_TIER_POOL_BANDWIDTH_PER_SECOND` = low (e.g. `2 MB`) so the throttle is visible below the link speed.
- `NYMNODE_FREE_TIER_BANDWIDTH_ALLOWANCE` = below the download size (e.g. `20 MB`) so the trial exhausts into the garden quickly.

All three default from the network-defaults constants; the `.env` already carries demo values. A redeploy applies changes on every start (no config wipe).

Known plain large-file endpoints (HTTPS): `https://proof.ovh.net/files/10Mb.dat` / `100Mb.dat` (`141.95.207.211`); `https://nym-bandwidth-monitoring.ops-d86.workers.dev/100mb.dat` (Cloudflare anycast `172.67.215.180` / `104.21.43.13` - whitelist both if using it as the purchase endpoint). The driver prints the resolved IP of each download so you know what to whitelist.

**Throttle (free vs paid):** download the same non-whitelisted file with and without `--free-tier` and compare the reported MB/s:
```bash
cargo run -p nym-free-tier-gateway-harness -- --gateway-http http://<ip>:8080 --gateway-ip <ip> \
    --free-tier --download https://proof.ovh.net/files/10Mb.dat     # throttled to the pool
cargo run -p nym-free-tier-gateway-harness -- --gateway-http http://<ip>:8080 --gateway-ip <ip> \
    --download https://proof.ovh.net/files/10Mb.dat                 # "paid" (mock zk-nym), full speed
```

**Exhaustion -> garden** (single `--free-tier` run; downloads run sequentially on one tunnel):
```bash
cargo run -p nym-free-tier-gateway-harness -- --gateway-http http://<ip>:8080 --gateway-ip <ip> --free-tier \
    --download https://nym-bandwidth-monitoring.ops-d86.workers.dev/100mb.dat \  # exhausts -> stalls (BLOCKED)
    --download https://proof.ovh.net/files/10Mb.dat \                            # non-whitelist -> BLOCKED
    --download https://<whitelisted-endpoint>/<file>                             # whitelist -> still OK
```

Watch the gateway logs for the transition markers (temporary demo logs, prefixed `>>>>> FREE-TIER:`): `ADMITTED ... TO RATE-LIMIT POOL`, `MOVING ... TO WALLED GARDEN`, `CONFINED ... AT REGISTRATION` (renewal), `RELEASED ... (upgraded to paid)`.

## The test signer keypair

Free-tier capability JWTs are verified offline against a single signer public key (the credential-proxy tier, not the upgrade-mode attester). The harness pins a throwaway keypair in `.env`:

- `NYMNODE_FREE_TIER_SIGNER_PUBKEY` - read by `nym-node` to verify tokens.
- `NYM_FREE_TIER_SIGNER_PRIVKEY` - TEST-ONLY, used by the Milestone B driver to mint valid tokens. `nym-node` never reads it.

Both are derived from the fixed seed `b"nym-free-tier-localtest-signer!!"`. The test `localtest_signer_keypair_matches_committed_env` in `common/free-tier-check` asserts the committed values match the seed, so they cannot silently drift. These keys are for this local harness only and must never be used on any real network.
