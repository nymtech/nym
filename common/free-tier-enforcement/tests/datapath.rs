// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Network-namespace datapath tests for the free-tier enforcement.
//!
//! They create netns + veths and drive `tc`/`nft`, so they need root + `NET_ADMIN`.
//! They are gated behind the `NYM_FREE_TIER_NETNS_TESTS` env var (NOT just `#[ignore]`
//! because CI runs the full `--ignored` suite) so they run only when explicitly opted in. Run
//! them via `netns/run.sh` (privileged container), or on a privileged Linux host:
//!   NYM_FREE_TIER_NETNS_TESTS=1 sudo -E cargo test -p nym-free-tier-enforcement \
//!       --test datapath -- --ignored --nocapture

// #![cfg(target_os = "linux")]
#![allow(clippy::panic)]

use nym_free_tier_enforcement::{
    CommandRunner, CommandSpec, EnforcementError, FreeTierEnforcement, PeerAddrs,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::Command;
use std::sync::Arc;

/// Run a command, returning its captured output.
fn sh(args: &[&str]) -> std::process::Output {
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {args:?}: {e}"))
}

/// Run a command and assert it succeeded (with helpful output on failure).
fn must(args: &[&str]) {
    let out = sh(args);
    assert!(
        out.status.success(),
        "command failed: {args:?}\n  stdout: {}\n  stderr: {}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim(),
    );
}

fn is_root() -> bool {
    String::from_utf8_lossy(&sh(&["id", "-u"]).stdout).trim() == "0"
}

/// Explicit opt-in gate. `#[ignore]` is not enough on its own: our CI runs the full
/// `--ignored` suite, so these privileged, container-only tests would run (and fail)
/// there. They run only when `NYM_FREE_TIER_NETNS_TESTS` is set, which `netns/run.sh`
/// does.
fn netns_tests_enabled() -> bool {
    std::env::var_os("NYM_FREE_TIER_NETNS_TESTS").is_some()
}

fn skip_unless_privileged() -> bool {
    if !netns_tests_enabled() {
        eprintln!(
            "SKIP: set NYM_FREE_TIER_NETNS_TESTS=1 (see netns/run.sh) to run the free-tier netns datapath tests"
        );
        return true;
    }
    if !is_root() {
        eprintln!("SKIP: free-tier netns tests require root (NET_ADMIN)");
        return true;
    }
    false
}

/// Deletes the listed namespaces on drop (also removes any veths inside them), so a
/// panicking test still cleans up.
struct NetnsGuard(&'static [&'static str]);

impl Drop for NetnsGuard {
    fn drop(&mut self) {
        for ns in self.0 {
            let _ = Command::new("ip").args(["netns", "del", ns]).status();
        }
    }
}

/// Shared forward-topology addressing (node forwards client <-> {allowed, other}).
const FWD_CLIENT_IP: &str = "10.0.1.2";
const FWD_ALLOWED_IP: &str = "10.0.2.2";
const FWD_OTHER_IP: &str = "10.0.3.2";
/// The node's veth facing the client - download (node -> client) egresses here, so it is
/// the interface the rate-limit pool shapes.
const FWD_CLIENT_DEV: &str = "ftvcn";

/// Build a "node forwards a client to an allowlisted + an other endpoint" topology into
/// the four given namespaces (fixed device names + addressing).
fn build_forward_topology(client: &str, node: &str, allowed: &str, other: &str) {
    for ns in [client, node, allowed, other] {
        must(&["ip", "netns", "add", ns]);
        must(&["ip", "netns", "exec", ns, "ip", "link", "set", "lo", "up"]);
    }

    let sysctl = |key: &str, val: &str| {
        must(&[
            "ip",
            "netns",
            "exec",
            node,
            "sh",
            "-c",
            &format!("echo {val} > /proc/sys/net/ipv4/{key}"),
        ]);
    };
    sysctl("ip_forward", "1");
    sysctl("conf/all/rp_filter", "0");
    sysctl("conf/default/rp_filter", "0");

    let link =
        |leaf: &str, node_dev: &str, leaf_dev: &str, node_cidr: &str, leaf_cidr: &str, gw: &str| {
            must(&[
                "ip", "link", "add", node_dev, "type", "veth", "peer", "name", leaf_dev,
            ]);
            must(&["ip", "link", "set", node_dev, "netns", node]);
            must(&["ip", "link", "set", leaf_dev, "netns", leaf]);
            must(&[
                "ip", "netns", "exec", node, "ip", "addr", "add", node_cidr, "dev", node_dev,
            ]);
            must(&[
                "ip", "netns", "exec", node, "ip", "link", "set", node_dev, "up",
            ]);
            must(&[
                "ip", "netns", "exec", leaf, "ip", "addr", "add", leaf_cidr, "dev", leaf_dev,
            ]);
            must(&[
                "ip", "netns", "exec", leaf, "ip", "link", "set", leaf_dev, "up",
            ]);
            must(&[
                "ip", "netns", "exec", leaf, "ip", "route", "add", "default", "via", gw,
            ]);
        };
    link(
        client,
        FWD_CLIENT_DEV,
        "ftvc",
        "10.0.1.1/24",
        "10.0.1.2/24",
        "10.0.1.1",
    );
    link(
        allowed,
        "ftvan",
        "ftva",
        "10.0.2.1/24",
        "10.0.2.2/24",
        "10.0.2.1",
    );
    link(
        other,
        "ftvon",
        "ftvo",
        "10.0.3.1/24",
        "10.0.3.2/24",
        "10.0.3.1",
    );
}

/// Ping `ip` from the given namespace with `count` packets of `size`-byte payload.
fn ping(ns: &str, ip: &str, count: &str, size: &str) -> bool {
    sh(&[
        "ip", "netns", "exec", ns, "ping", "-c", count, "-W", "2", "-s", size, ip,
    ])
    .status
    .success()
}

/// Bytes an HTB class has sent, parsed from `tc -s class show` (0 if not found).
fn class_sent_bytes(ns: &str, dev: &str, classid: &str) -> u64 {
    let out = sh(&[
        "ip", "netns", "exec", ns, "tc", "-s", "class", "show", "dev", dev,
    ]);
    let text = String::from_utf8_lossy(&out.stdout);
    let needle = format!(" {classid} ");
    let mut in_target = false;
    for line in text.lines() {
        if line.trim_start().starts_with("class ") {
            in_target = line.contains(&needle);
        } else if in_target {
            if let Some(pos) = line.find("Sent ") {
                return line[pos + 5..]
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            }
        }
    }
    0
}

/// Executes a manager's [`CommandSpec`] inside a network namespace (prefixing
/// `ip netns exec <ns>`), so the real `FreeTierEnforcement` drives a kernel we inspect.
struct NetnsRunner {
    ns: String,
}

impl CommandRunner for NetnsRunner {
    fn execute(&self, cmd: &CommandSpec, ignore_failure: bool) -> Result<(), EnforcementError> {
        let output = Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&self.ns)
            .arg(&cmd.program)
            .args(&cmd.args)
            .output()
            .map_err(|source| EnforcementError::Spawn {
                program: cmd.program.clone(),
                source,
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if ignore_failure {
                eprintln!("  (ignored) netns `{cmd}`: {stderr}");
                return Ok(());
            }
            return Err(EnforcementError::CommandFailed {
                command: cmd.rendered(),
                stderr,
            });
        }
        Ok(())
    }
}

/// v0 smoke test: prove the container/netns dev loop works end to end - two namespaces
/// joined by a veth pair can reach each other. Validates that the harness has
/// `NET_ADMIN` and that `ip netns`/veth behave, before we layer `tc`/`nft` on top.
#[test]
#[ignore = "needs linux + root + NET_ADMIN; run via netns/run.sh"]
fn veth_reachability_smoke() {
    if skip_unless_privileged() {
        return;
    }

    const CLIENT: &str = "ft_client";
    const SERVER: &str = "ft_server";
    let _guard = NetnsGuard(&[CLIENT, SERVER]);

    must(&["ip", "netns", "add", CLIENT]);
    must(&["ip", "netns", "add", SERVER]);

    must(&[
        "ip", "link", "add", "veth-c", "type", "veth", "peer", "name", "veth-s",
    ]);
    must(&["ip", "link", "set", "veth-c", "netns", CLIENT]);
    must(&["ip", "link", "set", "veth-s", "netns", SERVER]);

    for (ns, dev, addr) in [
        (CLIENT, "veth-c", "10.99.0.1/24"),
        (SERVER, "veth-s", "10.99.0.2/24"),
    ] {
        must(&[
            "ip", "netns", "exec", ns, "ip", "addr", "add", addr, "dev", dev,
        ]);
        must(&["ip", "netns", "exec", ns, "ip", "link", "set", dev, "up"]);
        must(&["ip", "netns", "exec", ns, "ip", "link", "set", "lo", "up"]);
    }

    must(&[
        "ip",
        "netns",
        "exec",
        CLIENT,
        "ping",
        "-c",
        "1",
        "-W",
        "2",
        "10.99.0.2",
    ]);
}

/// The full free-tier lifecycle through the REAL `FreeTierEnforcement` facade against a
/// live kernel: setup (one shared nft table + the HTB pool), then
///   admit -> pooled (download classified into `1:10`; whitelist exempt),
///   send_to_garden -> confined (only the whitelist reachable),
///   release -> unrestricted again.
/// Pool classification is checked via the tc class byte counters (deterministic, not
/// throughput); confinement via reachability.
#[test]
#[ignore = "needs linux + root + NET_ADMIN; run via netns/run.sh"]
fn free_tier_lifecycle_via_real_facade() {
    if skip_unless_privileged() {
        return;
    }

    const CLIENT: &str = "ftl_client";
    const NODE: &str = "ftl_node";
    const ALLOWED: &str = "ftl_allowed";
    const OTHER: &str = "ftl_other";
    let _guard = NetnsGuard(&[CLIENT, NODE, ALLOWED, OTHER]);

    build_forward_topology(CLIENT, NODE, ALLOWED, OTHER);

    // sanity: forwarding works before we enforce anything
    assert!(
        ping(CLIENT, FWD_ALLOWED_IP, "1", "56"),
        "baseline reach allowed"
    );
    assert!(
        ping(CLIENT, FWD_OTHER_IP, "1", "56"),
        "baseline reach other"
    );

    let enforcement = FreeTierEnforcement::new(
        FWD_CLIENT_DEV,
        125_000, // ~1 Mbit/s; irrelevant to a classification (not throughput) check
        vec![FWD_ALLOWED_IP.parse::<IpAddr>().unwrap()],
        Arc::new(NetnsRunner {
            ns: NODE.to_string(),
        }),
    );
    enforcement.setup().expect("setup should apply cleanly");
    // idempotent: a second setup (as on a startup reconcile) must not fail
    enforcement
        .setup()
        .expect("setup must be idempotent (re-runnable)");

    let peer = PeerAddrs {
        v4: FWD_CLIENT_IP.parse::<Ipv4Addr>().unwrap(),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 2),
    };

    // --- admit -> the pool -------------------------------------------------
    enforcement
        .admit(&peer)
        .expect("admit should apply cleanly");

    // (A) exemption: download FROM the whitelist is NOT pooled (counter stays 0)
    for _ in 0..5 {
        assert!(
            ping(CLIENT, FWD_ALLOWED_IP, "1", "1200"),
            "pooled peer still reaches the whitelist"
        );
    }
    assert_eq!(
        class_sent_bytes(NODE, FWD_CLIENT_DEV, "1:10"),
        0,
        "download from the whitelist must stay in the unlimited class, not the pool"
    );

    // (B) classification: download from a non-whitelist endpoint IS pooled
    for _ in 0..5 {
        assert!(
            ping(CLIENT, FWD_OTHER_IP, "1", "1200"),
            "pooled peer still reaches other endpoints (shaping paces, it does not drop)"
        );
    }
    assert!(
        class_sent_bytes(NODE, FWD_CLIENT_DEV, "1:10") > 0,
        "download to a pooled peer from a non-whitelist endpoint must land in the pool class"
    );

    // --- send_to_garden -> confined ---------------------------------------
    enforcement
        .send_to_garden(&peer)
        .expect("send_to_garden should apply cleanly");
    assert!(
        ping(CLIENT, FWD_ALLOWED_IP, "1", "56"),
        "garden: the whitelist (purchase) endpoint stays reachable"
    );
    assert!(
        !ping(CLIENT, FWD_OTHER_IP, "1", "56"),
        "garden: a non-whitelist endpoint is dropped"
    );

    // --- release -> unrestricted ------------------------------------------
    enforcement
        .release(&peer)
        .expect("release should apply cleanly");
    assert!(
        ping(CLIENT, FWD_OTHER_IP, "1", "56"),
        "released: a non-whitelist endpoint is reachable again"
    );
}
