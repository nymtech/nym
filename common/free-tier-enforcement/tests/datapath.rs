// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Network-namespace datapath tests for the free-tier enforcement primitives.
//!
//! They create netns + veths and drive `tc`/`iptables`, so they need root +
//! `NET_ADMIN`. They are gated behind the `NYM_FREE_TIER_NETNS_TESTS` env var
//! (NOT just `#[ignore]` - CI runs the full `--ignored` suite) so they run only
//! when explicitly opted in. Run them via `netns/run.sh` (privileged container),
//! or on a privileged Linux host:
//!   NYM_FREE_TIER_NETNS_TESTS=1 sudo -E cargo test -p nym-free-tier-enforcement \
//!       --test datapath -- --ignored --nocapture

#![cfg(target_os = "linux")]
#![allow(clippy::panic)]

use nym_free_tier_enforcement::{
    CommandRunner, CommandSpec, EnforcementError, PeerAddrs, RateLimitPool,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::Command;

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

/// Explicit opt-in gate. `#[ignore]` is not enough on its own: our CI runs the
/// full `--ignored` suite, so these privileged, container-only tests would run
/// (and fail) there. They run only when `NYM_FREE_TIER_NETNS_TESTS` is set,
/// which `netns/run.sh` does.
fn netns_tests_enabled() -> bool {
    std::env::var_os("NYM_FREE_TIER_NETNS_TESTS").is_some()
}

/// Deletes the listed namespaces on drop (also removes any veths inside them),
/// so a panicking test still cleans up.
struct NetnsGuard(&'static [&'static str]);

impl Drop for NetnsGuard {
    fn drop(&mut self) {
        for ns in self.0 {
            let _ = Command::new("ip").args(["netns", "del", ns]).status();
        }
    }
}

// ---------------------------------------------------------------------------
// task-4 rate-limit-pool validation: drive the REAL RateLimitPool in a namespace
// ---------------------------------------------------------------------------

/// Shared forward-topology addressing (node forwards client <-> {allowed, other}).
const FWD_CLIENT_IP: &str = "10.0.1.2";
const FWD_ALLOWED_IP: &str = "10.0.2.2";
const FWD_OTHER_IP: &str = "10.0.3.2";
/// The node's veth facing the client - download (node -> client) egresses here, so it
/// is the interface the rate-limit pool shapes.
const FWD_CLIENT_DEV: &str = "ftvcn";

/// Build a "node forwards a client to an allowlisted + an other endpoint" topology
/// into the four given namespaces (fixed device names + addressing).
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
/// `ip netns exec <ns>`), so the real `RateLimitPool` drives a kernel we can inspect.
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
                eprintln!("  (ignored) netns `{}`: {stderr}", cmd.rendered());
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

/// v0 smoke test: prove the container/netns dev loop works end to end -
/// two namespaces joined by a veth pair can reach each other. This validates
/// that the harness has `NET_ADMIN` and that `ip netns`/veth behave, before we
/// layer `tc` and `iptables` on top.
#[test]
#[ignore = "needs linux + root + NET_ADMIN; run via netns/run.sh"]
fn veth_reachability_smoke() {
    if !netns_tests_enabled() {
        eprintln!(
            "SKIP: set NYM_FREE_TIER_NETNS_TESTS=1 (see netns/run.sh) to run the free-tier netns datapath tests"
        );
        return;
    }
    if !is_root() {
        eprintln!("SKIP: free-tier netns tests require root (NET_ADMIN)");
        return;
    }

    const CLIENT: &str = "ft_client";
    const SERVER: &str = "ft_server";
    let _guard = NetnsGuard(&[CLIENT, SERVER]);

    // fresh namespaces
    must(&["ip", "netns", "add", CLIENT]);
    must(&["ip", "netns", "add", SERVER]);

    // a veth pair with one end in each namespace
    must(&[
        "ip", "link", "add", "veth-c", "type", "veth", "peer", "name", "veth-s",
    ]);
    must(&["ip", "link", "set", "veth-c", "netns", CLIENT]);
    must(&["ip", "link", "set", "veth-s", "netns", SERVER]);

    // address + bring up
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

    // the two ends can reach each other
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

/// v1: the realistic forward path. A "node" namespace forwards a "client" (peer)
/// to two destinations - an allowlisted "purchase endpoint" and any "other"
/// dest. Baseline: the client reaches both. Once the peer's forwarded traffic is
/// sent through the `NYM-GARDEN` allowlist chain, it reaches only the allowlisted
/// endpoint. The `tc` HTB pool is applied alongside to confirm the two coexist.
///
/// The node lives in its own namespace, so tearing the namespaces down removes
/// every veth, `tc` qdisc, and `iptables` rule in one shot. Namespace + link
/// names are distinct from the smoke test so the two can run concurrently.
#[test]
#[ignore = "needs linux + root + NET_ADMIN; run via netns/run.sh"]
fn forward_garden_allowlist() {
    if !netns_tests_enabled() {
        eprintln!(
            "SKIP: set NYM_FREE_TIER_NETNS_TESTS=1 (see netns/run.sh) to run the free-tier netns datapath tests"
        );
        return;
    }
    if !is_root() {
        eprintln!("SKIP: free-tier netns tests require root (NET_ADMIN)");
        return;
    }

    const CLIENT: &str = "ftg_client";
    const NODE: &str = "ftg_node";
    const ALLOWED: &str = "ftg_allowed";
    const OTHER: &str = "ftg_other";
    const CLIENT_IP: &str = "10.0.1.2";
    const ALLOWED_IP: &str = "10.0.2.2";
    const OTHER_IP: &str = "10.0.3.2";

    let _guard = NetnsGuard(&[CLIENT, NODE, ALLOWED, OTHER]);

    for ns in [CLIENT, NODE, ALLOWED, OTHER] {
        must(&["ip", "netns", "add", ns]);
        must(&["ip", "netns", "exec", ns, "ip", "link", "set", "lo", "up"]);
    }

    // the node forwards; disable reverse-path filtering for the forwarded paths
    let sysctl = |key: &str, val: &str| {
        must(&[
            "ip",
            "netns",
            "exec",
            NODE,
            "sh",
            "-c",
            &format!("echo {val} > /proc/sys/net/ipv4/{key}"),
        ]);
    };
    sysctl("ip_forward", "1");
    sysctl("conf/all/rp_filter", "0");
    sysctl("conf/default/rp_filter", "0");

    // wire a veth between the node and a leaf ns; leaf gets a default route via
    // the node. `node_cidr`/`leaf_cidr`/`gw` are passed as literals (no temporaries).
    let link =
        |leaf: &str, node_dev: &str, leaf_dev: &str, node_cidr: &str, leaf_cidr: &str, gw: &str| {
            must(&[
                "ip", "link", "add", node_dev, "type", "veth", "peer", "name", leaf_dev,
            ]);
            must(&["ip", "link", "set", node_dev, "netns", NODE]);
            must(&["ip", "link", "set", leaf_dev, "netns", leaf]);
            must(&[
                "ip", "netns", "exec", NODE, "ip", "addr", "add", node_cidr, "dev", node_dev,
            ]);
            must(&[
                "ip", "netns", "exec", NODE, "ip", "link", "set", node_dev, "up",
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
        CLIENT,
        "gvcn",
        "gvc",
        "10.0.1.1/24",
        "10.0.1.2/24",
        "10.0.1.1",
    );
    link(
        ALLOWED,
        "gvan",
        "gva",
        "10.0.2.1/24",
        "10.0.2.2/24",
        "10.0.2.1",
    );
    link(
        OTHER,
        "gvon",
        "gvo",
        "10.0.3.1/24",
        "10.0.3.2/24",
        "10.0.3.1",
    );

    let client_reaches = |ip: &str| -> bool {
        sh(&[
            "ip", "netns", "exec", CLIENT, "ping", "-c", "1", "-W", "2", ip,
        ])
        .status
        .success()
    };

    // baseline: the client reaches both destinations
    assert!(
        client_reaches(ALLOWED_IP),
        "baseline: client should reach the allowlisted endpoint"
    );
    assert!(
        client_reaches(OTHER_IP),
        "baseline: client should reach the other endpoint"
    );

    // apply the shared tc HTB pool on the node's peer-facing veth (shapes node->client)
    must(&[
        "ip", "netns", "exec", NODE, "tc", "qdisc", "add", "dev", "gvcn", "root", "handle", "1:",
        "htb", "default", "10",
    ]);
    must(&[
        "ip", "netns", "exec", NODE, "tc", "class", "add", "dev", "gvcn", "parent", "1:",
        "classid", "1:10", "htb", "rate", "10mbit", "ceil", "10mbit",
    ]);
    let qdisc = sh(&[
        "ip", "netns", "exec", NODE, "tc", "qdisc", "show", "dev", "gvcn",
    ]);
    assert!(
        String::from_utf8_lossy(&qdisc.stdout).contains("htb"),
        "the tc HTB pool should be present on the peer-facing veth"
    );

    // apply the walled garden: send the peer's forwarded traffic through the
    // NYM-GARDEN allowlist chain - allow the purchase endpoint, drop the rest
    must(&["ip", "netns", "exec", NODE, "iptables", "-N", "NYM-GARDEN"]);
    must(&[
        "ip",
        "netns",
        "exec",
        NODE,
        "iptables",
        "-A",
        "NYM-GARDEN",
        "-d",
        ALLOWED_IP,
        "-j",
        "ACCEPT",
    ]);
    must(&[
        "ip",
        "netns",
        "exec",
        NODE,
        "iptables",
        "-A",
        "NYM-GARDEN",
        "-j",
        "DROP",
    ]);
    must(&[
        "ip",
        "netns",
        "exec",
        NODE,
        "iptables",
        "-A",
        "FORWARD",
        "-s",
        CLIENT_IP,
        "-j",
        "NYM-GARDEN",
    ]);

    // garden: the client now reaches ONLY the allowlisted endpoint, full speed
    // to it (the tc pool + garden coexist)
    assert!(
        client_reaches(ALLOWED_IP),
        "garden: client should still reach the allowlisted (purchase) endpoint"
    );
    assert!(
        !client_reaches(OTHER_IP),
        "garden: client should NOT reach any endpoint outside the allowlist"
    );
}

/// Drive the REAL [`RateLimitPool`] against a namespace and prove task 4's one
/// unproven assumption: that `iptables CLASSIFY` actually steers a peer's download
/// into the HTB pool class, while the walled-garden whitelist stays in the unlimited
/// default class. Uses the class byte counters (`tc -s class show`) rather than
/// throughput, so it is a fast, deterministic classification check.
#[test]
#[ignore = "needs linux + root + NET_ADMIN; run via netns/run.sh"]
fn rate_limit_pool_classifies_download_and_exempts_whitelist() {
    if !netns_tests_enabled() {
        eprintln!(
            "SKIP: set NYM_FREE_TIER_NETNS_TESTS=1 (see netns/run.sh) to run the free-tier netns datapath tests"
        );
        return;
    }
    if !is_root() {
        eprintln!("SKIP: free-tier netns tests require root (NET_ADMIN)");
        return;
    }

    const CLIENT: &str = "ftc_client";
    const NODE: &str = "ftc_node";
    const ALLOWED: &str = "ftc_allowed";
    const OTHER: &str = "ftc_other";
    let _guard = NetnsGuard(&[CLIENT, NODE, ALLOWED, OTHER]);

    build_forward_topology(CLIENT, NODE, ALLOWED, OTHER);

    // sanity: forwarding works before we shape anything
    assert!(
        ping(CLIENT, FWD_ALLOWED_IP, "1", "56"),
        "baseline reach allowed"
    );
    assert!(
        ping(CLIENT, FWD_OTHER_IP, "1", "56"),
        "baseline reach other"
    );

    // the actual manager, executing its commands inside the node namespace: pool on
    // the client-facing device, with the allowlisted (purchase) endpoint exempt.
    let pool = RateLimitPool::new(
        FWD_CLIENT_DEV,
        125_000, // ~1 Mbit/s; irrelevant to a classification (not throughput) check
        vec![FWD_ALLOWED_IP.parse::<IpAddr>().unwrap()],
        Box::new(NetnsRunner {
            ns: NODE.to_string(),
        }),
    );
    pool.setup().expect("pool setup should apply cleanly");
    pool.add_peer(&PeerAddrs {
        v4: FWD_CLIENT_IP.parse::<Ipv4Addr>().unwrap(),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 2),
    })
    .expect("adding the peer should apply cleanly");

    // (A) exemption: download FROM the whitelisted endpoint must NOT be pooled. Ping
    // ONLY ALLOWED (so the pool counter starts clean) - it stays reachable, and the
    // pool class must have counted nothing.
    for _ in 0..5 {
        assert!(
            ping(CLIENT, FWD_ALLOWED_IP, "1", "1200"),
            "whitelisted endpoint stays reachable under the pool"
        );
    }
    assert_eq!(
        class_sent_bytes(NODE, FWD_CLIENT_DEV, "1:10"),
        0,
        "download from the whitelisted endpoint must stay in the unlimited class, not the pool"
    );

    // (B) classification: download from a NON-whitelisted endpoint must be pooled - the
    // per-peer CLASSIFY rule steers it into the HTB pool class.
    for _ in 0..5 {
        assert!(
            ping(CLIENT, FWD_OTHER_IP, "1", "1200"),
            "non-whitelisted endpoint stays reachable (shaping paces, it does not drop)"
        );
    }
    assert!(
        class_sent_bytes(NODE, FWD_CLIENT_DEV, "1:10") > 0,
        "download to a pooled peer from a non-whitelisted endpoint must land in the pool class"
    );
}
