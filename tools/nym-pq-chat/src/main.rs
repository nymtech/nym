// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod psk;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use nym_bin_common::logging::tracing_subscriber;
use nym_client_core_gateways_storage::GatewayDetails;
use nym_sdk::mixnet::{
    GatewaysDetailsStore, InputMessage, MixnetClient, MixnetClientBuilder, MixnetMessageSender,
    Recipient, StoragePaths, TransmissionLane,
};
use psk::PskCipher;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::Instant;
use tracing::{info, warn};

const STORAGE_DIR: &str = "nym-pq-chat-storage";
// sending only queues a message; give the client time to push it to the gateway before disconnecting
const FLUSH_GRACE: Duration = Duration::from_secs(5);

#[derive(Parser)]
#[command(
    name = "nym-pq-chat",
    version,
    about = "Pre-shared-key (post-quantum safe) end-to-end encrypted chat over the Nym mixnet"
)]
struct Cli {
    /// Directory holding key.secret, <me>.address.secret, <peer>.address.secret and nym-pq-chat-storage/
    #[arg(long, short, global = true, default_value = ".")]
    dir: PathBuf,

    /// Name of this machine; its mixnet address is saved to <dir>/<me>.address.secret
    #[arg(long, global = true, default_value = "me")]
    me: String,

    /// Name of the peer machine; its mixnet address is read from <dir>/<peer>.address.secret
    #[arg(long, global = true, default_value = "peer")]
    peer: String,

    /// Only use gateways reachable over TLS (wss://); decided at first connection and then persisted
    #[arg(long, global = true)]
    tls: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a fresh random pre-shared key into <dir>/key.secret (copy it to the peer OFFLINE)
    Keygen,
    /// Connect once to create this machine's mixnet identity and save it to <dir>/<me>.address.secret
    Init,
    /// Chat: every stdin line is encrypted and sent to the peer, received messages are decrypted
    Run {
        /// Also print the hex ciphertext of every sent and received message
        #[arg(long)]
        show_ciphertext: bool,
    },
    /// Encrypt stdin with key.secret and print hex ciphertext (offline test of the layer)
    Encrypt,
    /// Decrypt hex ciphertext from stdin with key.secret and print the plaintext
    Decrypt,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging();
    // dependencies enable several rustls providers; wss:// needs exactly one selected per process
    let _ = rustls::crypto::ring::default_provider().install_default();

    match cli.command {
        Command::Keygen => keygen(&cli.dir),
        Command::Init => init(&cli.dir, &cli.me, cli.tls).await,
        Command::Run { show_ciphertext } => {
            run(&cli.dir, &cli.me, &cli.peer, cli.tls, show_ciphertext).await
        }
        Command::Encrypt => encrypt(&cli.dir),
        Command::Decrypt => decrypt(&cli.dir),
    }
}

// chat goes to stdout, logs to stderr; RUST_LOG overrides the default (warn + our own info)
fn setup_logging() {
    use tracing_subscriber::filter::{EnvFilter, LevelFilter};
    use tracing_subscriber::prelude::*;

    let filter = if std::env::var_os("RUST_LOG").is_some() {
        EnvFilter::from_default_env()
    } else {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .parse_lossy("nym_pq_chat=info")
    };
    tracing_subscriber::registry()
        .with(nym_bin_common::logging::default_tracing_fmt_layer(
            std::io::stderr,
        ))
        .with(filter)
        .init();
}

fn key_path(dir: &Path) -> PathBuf {
    dir.join(psk::KEY_FILE)
}

fn address_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.address.secret"))
}

fn load_cipher(dir: &Path) -> Result<PskCipher> {
    PskCipher::from_key_file(&key_path(dir))
        .with_context(|| format!("no usable {} in {}; run `nym-pq-chat keygen` on one machine and copy the file to the other one offline", psk::KEY_FILE, dir.display()))
}

fn keygen(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let path = key_path(dir);
    psk::generate_key_file(&path)?;
    let cipher = PskCipher::from_key_file(&path)?;
    println!(
        "wrote {} (key fingerprint {})",
        path.display(),
        cipher.fingerprint()
    );
    println!("copy it to the peer machine OFFLINE (USB stick) - never send it over any network");
    Ok(())
}

async fn init(dir: &Path, me: &str, tls: bool) -> Result<()> {
    let client = connect(dir, me, tls).await?;
    client.disconnect().await;
    Ok(())
}

async fn connect(dir: &Path, me: &str, tls: bool) -> Result<MixnetClient> {
    let storage_dir = dir.join(STORAGE_DIR);
    std::fs::create_dir_all(&storage_dir)
        .with_context(|| format!("failed to create {}", storage_dir.display()))?;
    let paths = StoragePaths::new_from_dir(&storage_dir)?;
    if tls {
        ensure_stored_gateway_uses_tls(&paths).await?;
    }

    info!(
        "connecting to the Nym mixnet (client storage: {})",
        storage_dir.display()
    );
    let client = MixnetClientBuilder::new_with_default_storage(paths)
        .await?
        .force_tls(tls)
        .build()?
        .connect_to_mixnet()
        .await?;

    let address = client.nym_address().to_string();
    let path = address_path(dir, me);
    save_address(&path, &address)?;
    println!("our ({me}) nym address: {address}");
    println!(
        "saved to {}; copy it (same file name) into the peer machine's chat directory",
        path.display()
    );
    Ok(client)
}

// the SDK reuses a persisted gateway registration as-is, so `--tls` cannot upgrade a ws:// one
async fn ensure_stored_gateway_uses_tls(paths: &StoragePaths) -> Result<()> {
    let store = paths.on_disk_gateway_details_storage().await?;
    let Some(registration) = store.active_gateway().await?.registration else {
        return Ok(());
    };
    if let GatewayDetails::Remote(details) = &registration.details {
        let listener = &details.published_data.listeners.primary;
        if listener.scheme() != "wss" {
            bail!(
                "stored gateway {} was registered without TLS ({listener}); delete {STORAGE_DIR}/ and run `init --tls` again (this machine gets a new address)",
                registration.gateway_id()
            );
        }
    }
    Ok(())
}

fn save_address(path: &Path, address: &str) -> Result<()> {
    if let Ok(existing) = std::fs::read_to_string(path) {
        if existing.trim() == address {
            return Ok(());
        }
        std::fs::remove_file(path)
            .with_context(|| format!("failed to replace {}", path.display()))?;
    }
    psk::write_secret_file(path, format!("{address}\n").as_bytes())
}

fn load_peer(dir: &Path, peer: &str) -> Result<Recipient> {
    let path = address_path(dir, peer);
    let raw = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read peer address {}; run `nym-pq-chat --me {peer} init` on the peer machine and copy the file it saves here",
            path.display()
        )
    })?;
    raw.trim()
        .parse::<Recipient>()
        .map_err(|err| anyhow!("invalid peer address in {}: {err}", path.display()))
}

async fn run(dir: &Path, me: &str, peer: &str, tls: bool, show_ciphertext: bool) -> Result<()> {
    let cipher = load_cipher(dir)?;
    let peer_address = load_peer(dir, peer)?;
    let mut client = connect(dir, me, tls).await?;

    println!(
        "pre-shared key fingerprint: {} (must be identical on the peer)",
        cipher.fingerprint()
    );
    println!("peer {peer}: {peer_address}");
    println!("type a line and press Enter to send it; Ctrl-D or Ctrl-C quits");

    let sender = client.split_sender();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdin_open = true;
    let mut last_send: Option<Instant> = None;
    // set on stdin EOF: keep receiving until queued messages had time to leave
    let mut quit_at: Option<Instant> = None;

    loop {
        tokio::select! {
            line = lines.next_line(), if stdin_open => {
                let Some(line) = line.context("failed to read stdin")? else {
                    stdin_open = false;
                    quit_at = Some(last_send.map_or_else(Instant::now, |sent| sent + FLUSH_GRACE));
                    continue;
                };
                if line.is_empty() {
                    continue;
                }
                let ciphertext = cipher.encrypt(line.as_bytes())?;
                if show_ciphertext {
                    println!("[sending {} bytes of ciphertext: {}]", ciphertext.len(), hex::encode(&ciphertext));
                } else {
                    println!("[sending {} bytes of ciphertext]", ciphertext.len());
                }
                let message = InputMessage::new_regular(
                    peer_address,
                    ciphertext,
                    TransmissionLane::General,
                    sender.packet_type(),
                );
                sender.send(message).await.context("failed to send message")?;
                last_send = Some(Instant::now());
            }
            received = client.wait_for_messages() => {
                let Some(messages) = received else { break };
                for message in messages {
                    if show_ciphertext {
                        println!("[received {} bytes of ciphertext: {}]", message.message.len(), hex::encode(&message.message));
                    }
                    match cipher.decrypt(&message.message) {
                        Ok(plaintext) => println!("{peer}> {}", String::from_utf8_lossy(&plaintext)),
                        Err(err) => warn!("dropping {} byte message: {err}", message.message.len()),
                    }
                }
            }
            _ = async { match quit_at { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => break,
            _ = tokio::signal::ctrl_c() => break,
        }
    }

    println!("disconnecting...");
    client.disconnect().await;
    Ok(())
}

fn encrypt(dir: &Path) -> Result<()> {
    let cipher = load_cipher(dir)?;
    let mut plaintext = Vec::new();
    std::io::stdin()
        .read_to_end(&mut plaintext)
        .context("failed to read stdin")?;
    println!("{}", hex::encode(cipher.encrypt(&plaintext)?));
    Ok(())
}

fn decrypt(dir: &Path) -> Result<()> {
    let cipher = load_cipher(dir)?;
    let mut encoded = String::new();
    std::io::stdin()
        .read_to_string(&mut encoded)
        .context("failed to read stdin")?;
    let ciphertext = hex::decode(encoded.trim()).context("stdin is not hex ciphertext")?;
    let plaintext = cipher.decrypt(&ciphertext)?;
    std::io::stdout()
        .write_all(&plaintext)
        .context("failed to write stdout")?;
    Ok(())
}
