# Free-tier netns datapath tests

Linux-only integration tests that build a network-namespace topology and assert the free-tier enforcement datapath - the `tc` rate-limit pool and the `iptables` walled-garden allowlist. They create namespaces and drive `tc`/`iptables`, so they need root + `NET_ADMIN`.

They are gated behind the `NYM_FREE_TIER_NETNS_TESTS` env var, **not** just `#[ignore]`: our CI runs the full `--ignored` suite, so `#[ignore]` alone would let these privileged, container-only tests run (and fail) there. Without the env var they skip themselves; `run.sh` sets it.

## Run

On macOS (or any non-privileged host) - in a privileged Linux container:

```sh
./run.sh
```

`run.sh` prefers Docker (its `--privileged` netns path is well-trodden) and falls back to Apple's `container` runtime. It builds the image from the local `Dockerfile`, bind-mounts the repo, and runs the tests `--privileged`.

On a privileged Linux host (e.g. CI with a root runner) - directly, no container:

```sh
NYM_FREE_TIER_NETNS_TESTS=1 sudo -E cargo test -p nym-free-tier-enforcement --test datapath -- --ignored --nocapture
```

## Layout

- `Dockerfile` - toolchain + `iproute2`/`iptables`/`iputils-ping`; the build/test runs against the bind-mounted repo (a container-local `CARGO_TARGET_DIR` keeps Linux artifacts out of the host `./target`).
- `run.sh` - the runtime-detecting wrapper described above.
- the tests themselves live in `../tests/datapath.rs`.
