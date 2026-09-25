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
use std::io::{IsTerminal, Read, Write};
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

    /// Names of the peer machines (comma separated or repeated; --peers works too); each address is read from <dir>/<peer>.address.secret
    #[arg(
        long,
        alias = "peers",
        global = true,
        value_delimiter = ',',
        default_value = "peer"
    )]
    peer: Vec<String>,

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
    /// Chat: every stdin line is encrypted and sent to all peers, received messages are decrypted
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
    }
    // write next to the target and rename over it, so a failed write never leaves the file missing
    let tmp = path.with_extension("tmp.secret");
    let _ = std::fs::remove_file(&tmp);
    psk::write_secret_file(&tmp, format!("{address}\n").as_bytes())?;
    std::fs::rename(&tmp, path).with_context(|| format!("failed to replace {}", path.display()))
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

fn peer_names(names: &[String]) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for name in names.iter().map(|name| name.trim()) {
        if !name.is_empty() && !unique.iter().any(|seen| seen == name) {
            unique.push(name.to_owned());
        }
    }
    unique
}

fn load_peers(dir: &Path, names: &[String]) -> Result<Vec<(String, Recipient)>> {
    let names = peer_names(names);
    if names.is_empty() {
        bail!("no peer given; use --peer NAME[,NAME...]");
    }
    names
        .into_iter()
        .map(|name| load_peer(dir, &name).map(|address| (name, address)))
        .collect()
}

// chat payload inside the AEAD: "<sender>\n<line>", so a receiver with several peers knows who wrote
fn frame(sender: &str, line: &str) -> Vec<u8> {
    format!("{sender}\n{line}").into_bytes()
}

fn unframe(plaintext: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(plaintext);
    match text.split_once('\n') {
        Some((sender, line)) => (sender.to_owned(), line.to_owned()),
        None => ("?".to_owned(), text.into_owned()),
    }
}

// interactive terminal only: the input prompt is our own name, so it is clear which identity is typing
fn prompt_for(me: &str, interactive: bool) -> String {
    if interactive {
        format!("{me}: ")
    } else {
        String::new()
    }
}

fn show_prompt(prompt: &str) -> Result<()> {
    if !prompt.is_empty() {
        print!("{prompt}");
        std::io::stdout()
            .flush()
            .context("failed to flush stdout")?;
    }
    Ok(())
}

// move to the start of the prompt line and clear it, so incoming lines do not get appended to the prompt
fn clear_prompt(prompt: &str) {
    if !prompt.is_empty() {
        print!("\r\x1b[K");
    }
}

async fn run(
    dir: &Path,
    me: &str,
    peers: &[String],
    tls: bool,
    show_ciphertext: bool,
) -> Result<()> {
    let cipher = load_cipher(dir)?;
    let peers = load_peers(dir, peers)?;
    let mut client = connect(dir, me, tls).await?;

    println!(
        "pre-shared key fingerprint: {} (must be identical on the peers)",
        cipher.fingerprint()
    );
    for (name, address) in &peers {
        println!("peer {name}: {address}");
    }
    println!("type a line and press Enter to send it; Ctrl-D or Ctrl-C quits");
    let mut prompt = prompt_for(
        me,
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
    );
    show_prompt(&prompt)?;

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
                    if !prompt.is_empty() {
                        println!();
                        prompt.clear();
                    }
                    stdin_open = false;
                    quit_at = Some(last_send.map_or_else(Instant::now, |sent| sent + FLUSH_GRACE));
                    continue;
                };
                if line.is_empty() {
                    show_prompt(&prompt)?;
                    continue;
                }
                let ciphertext = cipher.encrypt(&frame(me, &line))?;
                if show_ciphertext {
                    println!("[sending {} bytes of ciphertext: {}]", ciphertext.len(), hex::encode(&ciphertext));
                } else {
                    println!("[sending {} bytes of ciphertext]", ciphertext.len());
                }
                for (_, address) in &peers {
                    let message = InputMessage::new_regular(
                        *address,
                        ciphertext.clone(),
                        TransmissionLane::General,
                        sender.packet_type(),
                    );
                    sender.send(message).await.context("failed to send message")?;
                }
                last_send = Some(Instant::now());
                show_prompt(&prompt)?;
            }
            received = client.wait_for_messages() => {
                let Some(messages) = received else { break };
                clear_prompt(&prompt);
                for message in messages {
                    if show_ciphertext {
                        println!("[received {} bytes of ciphertext: {}]", message.message.len(), hex::encode(&message.message));
                    }
                    match cipher.decrypt(&message.message) {
                        Ok(plaintext) => {
                            let (from, text) = unframe(&plaintext);
                            println!("{from}> {text}");
                        }
                        Err(err) => warn!("dropping {} byte message: {err}", message.message.len()),
                    }
                }
                show_prompt(&prompt)?;
            }
            _ = async { match quit_at { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => break,
            _ = tokio::signal::ctrl_c() => break,
        }
    }

    if !prompt.is_empty() {
        println!();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let (from, text) = unframe(&frame("tuxi", "hello, all three of you"));
        assert_eq!(from, "tuxi");
        assert_eq!(text, "hello, all three of you");
    }

    #[test]
    fn unframe_without_sender_tag() {
        let (from, text) = unframe(b"plain line");
        assert_eq!(from, "?");
        assert_eq!(text, "plain line");
    }

    #[test]
    fn prompt_is_own_name_only_when_interactive() {
        assert_eq!(prompt_for("tuxi", true), "tuxi: ");
        assert!(prompt_for("tuxi", false).is_empty());
    }

    #[test]
    fn peer_names_are_trimmed_and_deduplicated() {
        let names = ["tuxi", " asgard", "tuxi", "", "dockaws "].map(str::to_owned);
        assert_eq!(peer_names(&names), ["tuxi", "asgard", "dockaws"]);
    }

    #[test]
    fn load_peers_rejects_empty_list() {
        assert!(load_peers(Path::new("."), &[String::new()]).is_err());
    }

    #[test]
    fn save_address_replaces_atomically() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = address_path(dir.path(), "host1");
        save_address(&path, "first")?;
        save_address(&path, "first")?;
        save_address(&path, "second")?;
        assert_eq!(std::fs::read_to_string(&path)?, "second\n");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())?
            .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
            .collect();
        assert_eq!(leftovers, ["host1.address.secret"]);
        Ok(())
    }
}
