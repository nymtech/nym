//! End-to-end test of the `echo-service` / `echo-client` examples against
//! the live mainnet mixnet.
//!
//! Expensive and network-dependent, so it is gated: the test body only runs
//! when the `NYM_SDK_MAINNET_INTEGRATION_TESTS` environment variable is set.
//! Without it the test prints a skip message and returns immediately, which
//! keeps plain `cargo test` runs free of network access.
//!
//! ```sh
//! cargo build --package nym-sdk --examples
//! NYM_SDK_MAINNET_INTEGRATION_TESTS=1 cargo test --package nym-sdk \
//!     --test echo_example_integration -- --nocapture
//! ```

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const GATE_VAR: &str = "NYM_SDK_MAINNET_INTEGRATION_TESTS";

/// How long the service gets to connect to the mixnet and print its address.
const SERVICE_STARTUP_TIMEOUT: Duration = Duration::from_secs(180);
/// How long the client gets for the full mainnet round trip.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(300);

// These prefixes are printed by the examples; both sides carry a comment
// pointing back at this test.
const ADDRESS_LINE_PREFIX: &str = "echo-service listening on: ";
const REPLY_LINE_PREFIX: &str = "received reply: ";

/// Kills the spawned service on every exit path, including panics.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Examples are built next to the test binary: the test runs from
/// `target/<profile>/deps/`, the examples live in `target/<profile>/examples/`.
fn example_binary(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary has a path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("examples");
    path.push(name);
    assert!(
        path.exists(),
        "{} not found at {} - build it first with `cargo build --package nym-sdk --examples`",
        name,
        path.display(),
    );
    path
}

/// Echo every stdout line with a tag as it arrives — so failures still show
/// what both processes said — and forward it for inspection. The channel
/// closes when the process's stdout reaches EOF.
fn stream_stdout(tag: &'static str, stdout: ChildStdout) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            println!("[{tag}] {line}");
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

#[test]
fn echo_examples_round_trip_on_mainnet() {
    if std::env::var(GATE_VAR).is_err() {
        println!("skipping mainnet integration test: {GATE_VAR} is not set");
        return;
    }

    // Start the service and capture its stdout.
    let mut service = KillOnDrop(
        Command::new(example_binary("echo-service"))
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn echo-service"),
    );
    let service_lines = stream_stdout(
        "echo-service",
        service.0.stdout.take().expect("stdout was piped"),
    );

    // Wait for the printed nym address.
    let deadline = Instant::now() + SERVICE_STARTUP_TIMEOUT;
    let address = loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .expect("echo-service did not print its address within the startup timeout");
        let line = service_lines
            .recv_timeout(remaining)
            .expect("echo-service exited or timed out before printing its address");
        if let Some(address) = line.strip_prefix(ADDRESS_LINE_PREFIX) {
            break address.trim().to_string();
        }
    };
    println!("service is up at {address}, starting client");

    // Run the client against it, bounded by a timeout.
    let mut client = Command::new(example_binary("echo-client"))
        .arg(&address)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn echo-client");
    let client_lines = stream_stdout(
        "echo-client",
        client.stdout.take().expect("stdout was piped"),
    );

    let deadline = Instant::now() + CLIENT_TIMEOUT;
    let client_status = loop {
        match client.try_wait().expect("failed to poll echo-client") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = client.kill();
                let _ = client.wait();
                panic!("echo-client did not finish within {CLIENT_TIMEOUT:?}");
            }
            None => std::thread::sleep(Duration::from_millis(250)),
        }
    };
    assert!(
        client_status.success(),
        "echo-client failed: {client_status}"
    );

    // Validate the reply the client printed. The client has exited, so its
    // stdout is at EOF and the channel drains completely.
    let reply_json = client_lines
        .into_iter()
        .find_map(|line| line.strip_prefix(REPLY_LINE_PREFIX).map(str::to_string))
        .expect("echo-client never printed a reply");

    let reply: serde_json::Value =
        serde_json::from_str(&reply_json).expect("reply is not valid JSON");
    assert_eq!(reply["message"], "hello");

    let timestamp = reply["timestamp_utc"]
        .as_str()
        .expect("timestamp_utc is not a string");
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
        .expect("timestamp_utc is not RFC 3339");

    let request_id = reply["request_id"]
        .as_str()
        .expect("request_id is not a string");
    uuid::Uuid::parse_str(request_id).expect("request_id is not a valid UUID");

    println!("mainnet round trip verified: {reply_json}");
}
