// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod psk;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use clap::{Parser, Subcommand};
use nym_bin_common::logging::tracing_subscriber;
use nym_client_core_gateways_storage::GatewayDetails;
use nym_sdk::mixnet::{
    GatewaysDetailsStore, InputMessage, LaneQueueLengths, MixnetClient, MixnetClientBuilder,
    MixnetClientSender, MixnetMessageSender, Recipient, StoragePaths, TransmissionLane,
};
use psk::PskCipher;
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::Instant;
use tracing::{info, warn};

const STORAGE_DIR: &str = "nym-pq-chat-storage";
// sending only queues a message; give the client time to push it to the gateway before disconnecting
const FLUSH_GRACE: Duration = Duration::from_secs(5);

/// Chat lines are meant to be short; longer content goes through `/file:`.
const MAX_LINE_BYTES: usize = 4096;
/// `/file: <path>` typed in the chat sends that file.
const FILE_COMMAND: &str = "/file:";
// inside the AEAD a file chunk body is "/file <id> <index> <count> <size> <path>\n<bytes>"
const FILE_TAG: &str = "/file ";
/// Photos and short compressed videos fit; anything bigger is refused.
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
/// Received files are saved as `name.ext`, `name-2.ext`, ... in the current directory.
const MAX_NAME_ATTEMPTS: u32 = 1000;
/// A file that could not be saved is shown as base64 only up to this size (about 1.33 MiB of text).
const MAX_INLINE_FILE: usize = 1024 * 1024;
// sender-side scheduling only: chat lines are never queued behind file chunks
const FILE_LANE: TransmissionLane = TransmissionLane::ConnectionId(0x7071_6368_6174);
// how many file packets to keep queued inside the client, topped up every FILE_TICK
const FILE_QUEUE_TARGET: usize = 256;
const FILE_TICK: Duration = Duration::from_millis(100);
// the client publishes a lane's length only when it pops from it and drops the entry once empty, so a
// small batch popped between two ticks is never seen; after this long without news treat it as sent
const FILE_SETTLE: Duration = Duration::from_secs(1);
/// Progress of a file transfer is reported every 10% or after this long, whichever comes first.
const PROGRESS_INTERVAL: Duration = Duration::from_secs(30);

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
    /// Chat: every stdin line is encrypted and sent to all peers, received messages are decrypted; `/file: <path>` sends a file
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

fn main() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start the async runtime")?;
    let result = runtime.block_on(async_main());
    // stdin is read on a blocking thread that only returns on input; dropping the runtime would wait for it
    runtime.shutdown_background();
    result
}

async fn async_main() -> Result<()> {
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

// chat payload inside the AEAD: "<sender>\n<body>", so a receiver with several peers knows who wrote
fn frame(sender: &str, line: &str) -> Vec<u8> {
    format!("{sender}\n{line}").into_bytes()
}

fn split_sender(plaintext: &[u8]) -> (String, &[u8]) {
    match plaintext.iter().position(|&b| b == b'\n') {
        Some(at) => (
            String::from_utf8_lossy(&plaintext[..at]).into_owned(),
            &plaintext[at + 1..],
        ),
        None => ("?".to_owned(), plaintext),
    }
}

// peers only prove they hold key.secret; never let their text drive the terminal
fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}

fn expand_home(path: &str) -> String {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => Path::new(&home).join(rest).display().to_string(),
        _ => path.to_owned(),
    }
}

// fixed-width fields, so the header length (and thus the chunk capacity) is the same for every chunk
fn file_header(sender: &str, id: u32, index: u32, count: u32, size: u64, path: &str) -> String {
    format!("{sender}\n{FILE_TAG}{id:08x} {index:08x} {count:08x} {size:016x} {path}\n")
}

/// File bytes per chunk such that every chunk message is exactly one padding block, like a chat line.
fn file_chunk_capacity(sender: &str, path: &str) -> Result<usize> {
    psk::MAX_BLOCK_MESSAGE
        .checked_sub(file_header(sender, 0, 0, 0, 0, path).len())
        .filter(|capacity| *capacity >= 64)
        .ok_or_else(|| anyhow!("file path too long"))
}

struct FileChunk<'a> {
    id: u32,
    index: u32,
    count: u32,
    size: u64,
    path: &'a str,
    data: &'a [u8],
}

fn parse_file_chunk(body: &[u8]) -> Result<FileChunk<'_>> {
    let rest = body
        .strip_prefix(FILE_TAG.as_bytes())
        .ok_or_else(|| anyhow!("not a file chunk"))?;
    let end = rest
        .iter()
        .position(|&b| b == b'\n')
        .ok_or_else(|| anyhow!("missing file header"))?;
    let header = std::str::from_utf8(&rest[..end]).context("file header is not utf-8")?;
    let data = &rest[end + 1..];
    let mut fields = header.splitn(5, ' ');
    let mut hex = |what: &str| -> Result<u64> {
        let field = fields
            .next()
            .ok_or_else(|| anyhow!("file header lacks {what}"))?;
        u64::from_str_radix(field, 16).with_context(|| format!("bad {what} in file header"))
    };
    let id = u32::try_from(hex("id")?)?;
    let index = u32::try_from(hex("index")?)?;
    let count = u32::try_from(hex("count")?)?;
    let size = hex("size")?;
    let path = fields
        .next()
        .ok_or_else(|| anyhow!("file header lacks path"))?;
    if count == 0 || index >= count {
        bail!("chunk {index} of {count} is out of range");
    }
    if size > MAX_FILE_SIZE || u64::from(count) > MAX_FILE_SIZE {
        bail!("file too large ({size} bytes in {count} chunks)");
    }
    if path.is_empty() || path.chars().any(char::is_control) {
        bail!("invalid file path");
    }
    if data.len() > psk::MAX_BLOCK_MESSAGE {
        bail!("chunk too large ({} bytes)", data.len());
    }
    Ok(FileChunk {
        id,
        index,
        count,
        size,
        path,
        data,
    })
}

/// Progress reports of one transfer: every 10% or every `PROGRESS_INTERVAL`, whichever comes first, never once complete.
struct Progress {
    reported: u32,
    last: Instant,
}

impl Progress {
    fn new() -> Self {
        Self::at(Instant::now())
    }

    fn at(now: Instant) -> Self {
        Self {
            reported: 0,
            last: now,
        }
    }

    fn step(&mut self, done: u32, total: u32) -> Option<u32> {
        self.step_at(done, total, Instant::now())
    }

    fn step_at(&mut self, done: u32, total: u32, now: Instant) -> Option<u32> {
        let percent = u32::try_from(u64::from(done) * 100 / u64::from(total.max(1))).unwrap_or(100);
        let next_step = percent / 10 > self.reported / 10;
        if done < total && (next_step || now.duration_since(self.last) >= PROGRESS_INTERVAL) {
            self.reported = percent;
            self.last = now;
            Some(percent)
        } else {
            None
        }
    }
}

/// A file being sent, streamed from disk chunk by chunk.
struct FileSend {
    file: File,
    path: String,
    id: u32,
    size: u64,
    count: u32,
    capacity: usize,
    next: u32,
    progress: Progress,
}

impl FileSend {
    fn open(sender: &str, path: &str) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("cannot open {path}"))?;
        let metadata = file
            .metadata()
            .with_context(|| format!("cannot stat {path}"))?;
        if !metadata.is_file() {
            bail!("{path} is not a regular file");
        }
        let size = metadata.len();
        if size > MAX_FILE_SIZE {
            bail!("{path} is {size} bytes, the limit is {MAX_FILE_SIZE} bytes");
        }
        let capacity = file_chunk_capacity(sender, path)?;
        let count = u32::try_from(size.div_ceil(capacity as u64).max(1))?;
        Ok(Self {
            file,
            path: path.to_owned(),
            id: rand::random(),
            size,
            count,
            capacity,
            next: 0,
            progress: Progress::new(),
        })
    }

    /// Framed plaintext of the next chunk, `None` once every chunk was produced.
    fn next_chunk(&mut self, sender: &str) -> Result<Option<Vec<u8>>> {
        if self.next >= self.count {
            return Ok(None);
        }
        let mut frame = file_header(
            sender, self.id, self.next, self.count, self.size, &self.path,
        )
        .into_bytes();
        let start = frame.len();
        frame.resize(start + self.capacity, 0);
        let mut filled = 0;
        while filled < self.capacity {
            let read = self
                .file
                .read(&mut frame[start + filled..])
                .with_context(|| format!("cannot read {}", self.path))?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        frame.truncate(start + filled);
        self.next += 1;
        Ok(Some(frame))
    }

    fn progress(&mut self) -> Option<u32> {
        self.progress.step(self.next, self.count)
    }

    fn exhausted(&self) -> bool {
        self.next >= self.count
    }
}

/// A file being received; chunks may arrive in any order.
struct FileReceive {
    path: String,
    size: u64,
    count: u32,
    chunks: BTreeMap<u32, Vec<u8>>,
    progress: Progress,
}

type Incoming = HashMap<(String, u32), FileReceive>;
/// Progress percentage to report (if any) and the finished file (path, contents) once complete.
type ChunkOutcome = (Option<u32>, Option<(String, Vec<u8>)>);

/// Stores one chunk; returns the whole file once the last chunk arrived.
fn store_chunk(incoming: &mut Incoming, from: &str, chunk: &FileChunk) -> Result<ChunkOutcome> {
    let key = (from.to_owned(), chunk.id);
    let entry = incoming.entry(key.clone()).or_insert_with(|| FileReceive {
        path: chunk.path.to_owned(),
        size: chunk.size,
        count: chunk.count,
        chunks: BTreeMap::new(),
        progress: Progress::new(),
    });
    if entry.path != chunk.path || entry.size != chunk.size || entry.count != chunk.count {
        bail!("chunk header does not match the transfer it belongs to");
    }
    entry.chunks.insert(chunk.index, chunk.data.to_vec());
    let received = u32::try_from(entry.chunks.len()).unwrap_or(u32::MAX);
    let percent = entry.progress.step(received, entry.count);
    if received < entry.count {
        return Ok((percent, None));
    }
    let Some(file) = incoming.remove(&key) else {
        return Ok((percent, None));
    };
    let mut data = Vec::with_capacity(usize::try_from(file.size).unwrap_or(0));
    for chunk in file.chunks.into_values() {
        data.extend_from_slice(&chunk);
    }
    if data.len() as u64 != file.size {
        bail!(
            "file {} reassembled to {} bytes, expected {}",
            file.path,
            data.len(),
            file.size
        );
    }
    Ok((percent, Some((file.path, data))))
}

fn write_new_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(data).inspect_err(|_| {
        // do not leave a truncated file behind under the name we just claimed
        let _ = std::fs::remove_file(path);
    })
}

/// Only the file name of the peer-supplied path is used, in the current directory: `./name.ext`, then
/// `./name-2.ext`, `./name-3.ext`, ... (never overwriting); any other write error is returned.
fn save_received_file(path: &str, data: &[u8]) -> Result<PathBuf, String> {
    let Some(name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
        return Err("no usable file name in the received path".to_owned());
    };
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    for n in 1..=MAX_NAME_ATTEMPTS {
        let candidate = if n == 1 {
            name.to_owned()
        } else {
            format!("{stem}-{n}{ext}")
        };
        let local = Path::new(".").join(candidate);
        match write_new_file(&local, data) {
            Ok(()) => return Ok(local),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("cannot write {}: {err}", local.display())),
        }
    }
    Err(format!(
        "{name} and {MAX_NAME_ATTEMPTS} numbered variants already exist"
    ))
}

/// Lines to show for one decrypted message: a chat line, or file progress / delivery (nothing for most chunks).
fn receive_lines(incoming: &mut Incoming, plaintext: &[u8]) -> Vec<String> {
    let (from, body) = split_sender(plaintext);
    if !body.starts_with(FILE_TAG.as_bytes()) {
        return vec![format!(
            "{}> {}",
            sanitize(&from),
            sanitize(&String::from_utf8_lossy(body))
        )];
    }
    let mut lines = Vec::new();
    match parse_file_chunk(body).and_then(|chunk| store_chunk(incoming, &from, &chunk)) {
        Ok((percent, complete)) => {
            if let Some(percent) = percent {
                lines.push(format!("[{}> file: {percent}% received]", sanitize(&from)));
            }
            if let Some((path, data)) = complete {
                lines.extend(delivery_report(
                    &from,
                    &path,
                    &data,
                    save_received_file(&path, &data),
                ));
            }
        }
        Err(err) => warn!("dropping file chunk from {}: {err:#}", sanitize(&from)),
    }
    lines
}

/// Where the file was saved, or the base64 fallback (small files only; bigger ones are discarded).
fn delivery_report(
    from: &str,
    path: &str,
    data: &[u8],
    saved: Result<PathBuf, String>,
) -> Vec<String> {
    let (from, shown, size) = (sanitize(from), sanitize(path), data.len());
    match saved {
        Ok(saved) => vec![format!(
            "{from}> received-file: {shown} ({size} bytes) saved to {}",
            saved.display()
        )],
        Err(err) if size > MAX_INLINE_FILE => vec![format!(
            "{from}> received-file: {shown} ({size} bytes) not saved ({err}); lost: not shown as base64 because it is bigger than {MAX_INLINE_FILE} bytes"
        )],
        Err(err) => vec![
            format!(
                "{from}> received-file: {shown} ({size} bytes) not saved ({err}); base64 follows"
            ),
            String::new(),
            base64::engine::general_purpose::STANDARD.encode(data),
            String::new(),
        ],
    }
}

async fn send_to_peers(
    sender: &MixnetClientSender,
    peers: &[(String, Recipient)],
    ciphertext: &[u8],
    lane: TransmissionLane,
) -> Result<()> {
    for (_, address) in peers {
        let message =
            InputMessage::new_regular(*address, ciphertext.to_vec(), lane, sender.packet_type());
        sender
            .send(message)
            .await
            .context("failed to send message")?;
    }
    Ok(())
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

// never redraw: leave the prompt line (and whatever is typed on it) as it is, output goes on the next line
fn leave_prompt(prompt: &str) {
    if !prompt.is_empty() {
        println!();
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
    println!(
        "type a line and press Enter to send it; {FILE_COMMAND} <path> sends a file; Ctrl-D or Ctrl-C quits"
    );
    let mut prompt = prompt_for(
        me,
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
    );
    show_prompt(&prompt)?;

    let sender = client.split_sender();
    let lane_lengths: LaneQueueLengths = client.shared_lane_queue_lengths();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdin_open = true;
    let mut last_send: Option<Instant> = None;
    // set on stdin EOF (once no file is in flight): keep receiving until queued messages had time to leave
    let mut quit_at: Option<Instant> = None;
    let mut file_send: Option<FileSend> = None;
    let mut incoming = Incoming::new();
    let mut file_tick = tokio::time::interval(FILE_TICK);
    // one long-lived listener: a fresh ctrl_c() per iteration misses a signal arriving between polls
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);
    // estimate of our file packets still queued inside the client, see FILE_SETTLE
    let mut file_in_flight = 0usize;
    let mut file_lane_published: Option<usize> = None;
    let mut file_last_push: Option<Instant> = None;

    loop {
        tokio::select! {
            line = lines.next_line(), if stdin_open => {
                let Some(line) = line.context("failed to read stdin")? else {
                    if !prompt.is_empty() {
                        println!();
                        prompt.clear();
                    }
                    stdin_open = false;
                    continue;
                };
                if line.is_empty() {
                    show_prompt(&prompt)?;
                    continue;
                }
                if let Some(path) = line.strip_prefix(FILE_COMMAND) {
                    let path = expand_home(path.trim());
                    if let Some(current) = &file_send {
                        println!("[still sending {}; one file at a time]", current.path);
                    } else {
                        match FileSend::open(me, &path) {
                            Ok(send) => {
                                println!(
                                    "[sending file {path} ({} bytes) as {} messages of {} bytes]",
                                    send.size, send.count, psk::PAD_BLOCK + psk::OVERHEAD
                                );
                                file_send = Some(send);
                            }
                            Err(err) => println!("[not sent: {err:#}]"),
                        }
                    }
                    show_prompt(&prompt)?;
                    continue;
                }
                if line.len() > MAX_LINE_BYTES {
                    println!(
                        "[not sent: line is {} bytes, the limit is {MAX_LINE_BYTES}; use {FILE_COMMAND} <path> instead]",
                        line.len()
                    );
                    show_prompt(&prompt)?;
                    continue;
                }
                let ciphertext = cipher.encrypt(&frame(me, &line))?;
                if show_ciphertext {
                    println!("[sending {} bytes of ciphertext: {}]", ciphertext.len(), hex::encode(&ciphertext));
                } else {
                    println!("[sending {} bytes of ciphertext]", ciphertext.len());
                }
                send_to_peers(&sender, &peers, &ciphertext, TransmissionLane::General).await?;
                last_send = Some(Instant::now());
                show_prompt(&prompt)?;
            }
            _ = file_tick.tick() => {
                let published = lane_lengths.get(&FILE_LANE);
                if published != file_lane_published {
                    // the client popped from our lane since the last tick: its count is authoritative
                    file_lane_published = published;
                    file_in_flight = published.unwrap_or(0);
                } else if published.is_none() && file_last_push.is_some_and(|at| at.elapsed() > FILE_SETTLE) {
                    file_in_flight = 0;
                }
                let mut finished = false;
                if let Some(send) = file_send.as_mut() {
                    while !send.exhausted() && file_in_flight < FILE_QUEUE_TARGET {
                        let Some(frame) = send.next_chunk(me)? else { break };
                        send_to_peers(&sender, &peers, &cipher.encrypt(&frame)?, FILE_LANE).await?;
                        file_in_flight += peers.len();
                        file_last_push = Some(Instant::now());
                        if let Some(percent) = send.progress() {
                            leave_prompt(&prompt);
                            println!("[file {}: {percent}% sent]", send.path);
                            show_prompt(&prompt)?;
                        }
                    }
                    if send.exhausted() && file_in_flight == 0 {
                        leave_prompt(&prompt);
                        println!("[file {} ({} bytes) sent as {} messages]", send.path, send.size, send.count);
                        show_prompt(&prompt)?;
                        last_send = Some(Instant::now());
                        finished = true;
                    }
                } else if !stdin_open && quit_at.is_none() && file_in_flight == 0 {
                    quit_at = Some(last_send.map_or_else(Instant::now, |sent| sent + FLUSH_GRACE));
                }
                if finished {
                    file_send = None;
                }
            }
            received = client.wait_for_messages() => {
                let Some(messages) = received else { break };
                let mut lines = Vec::new();
                for message in messages {
                    if show_ciphertext {
                        lines.push(format!("[received {} bytes of ciphertext: {}]", message.message.len(), hex::encode(&message.message)));
                    }
                    match cipher.decrypt(&message.message) {
                        Ok(plaintext) => lines.extend(receive_lines(&mut incoming, &plaintext)),
                        Err(err) => warn!("dropping {} byte message: {err}", message.message.len()),
                    }
                }
                // only when there is output: most file chunks are silent and must not disturb typing
                if !lines.is_empty() {
                    leave_prompt(&prompt);
                    for line in lines {
                        println!("{line}");
                    }
                    show_prompt(&prompt)?;
                }
            }
            _ = async { match quit_at { Some(at) => tokio::time::sleep_until(at).await, None => std::future::pending().await } } => break,
            _ = &mut ctrl_c => break,
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
        let framed = frame("tuxi", "hello, all three of you");
        let (from, body) = split_sender(&framed);
        assert_eq!(from, "tuxi");
        assert_eq!(body, b"hello, all three of you");
    }

    #[test]
    fn unframe_without_sender_tag() {
        let (from, body) = split_sender(b"plain line");
        assert_eq!(from, "?");
        assert_eq!(body, b"plain line");
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

    #[test]
    fn long_lines_survive_framing_and_encryption() -> Result<()> {
        let dir = tempfile::tempdir()?;
        keygen(dir.path())?;
        let cipher = load_cipher(dir.path())?;
        // below one padding block, just under/over a mixnet packet, the 4 KiB line limit, and beyond
        for len in [1, 1000, 1500, 2100, MAX_LINE_BYTES, 32 * 1024] {
            let line: String = (0..len)
                .map(|i| char::from(b'a' + (i % 26) as u8))
                .collect();
            let plaintext = frame("tuxi", &line);
            let ciphertext = cipher.encrypt(&plaintext)?;
            assert_eq!(
                ciphertext.len(),
                psk::padded_len(plaintext.len()) + psk::OVERHEAD
            );
            let decrypted = cipher.decrypt(&ciphertext)?;
            let (from, body) = split_sender(&decrypted);
            assert_eq!(from, "tuxi");
            assert_eq!(body, line.as_bytes());
            assert!(!body.starts_with(FILE_TAG.as_bytes()));
        }
        Ok(())
    }

    #[test]
    fn line_limit_is_four_kib_and_a_file_chunk_is_one_block() {
        assert_eq!(MAX_LINE_BYTES, 4096);
        assert_eq!(
            psk::padded_len(frame("tuxi", "hi").len()) + psk::OVERHEAD,
            1065
        );
        let capacity = file_chunk_capacity("tuxi", "photo.jpg").unwrap_or(0);
        let header = file_header("tuxi", u32::MAX, u32::MAX, u32::MAX, u64::MAX, "photo.jpg");
        assert_eq!(header.len() + capacity, psk::MAX_BLOCK_MESSAGE);
        assert_eq!(psk::padded_len(header.len() + capacity), psk::PAD_BLOCK);
    }

    #[test]
    fn sanitize_escapes_control_characters_only() {
        assert_eq!(sanitize("plain zażółć"), "plain zażółć");
        assert_eq!(sanitize("a\x1b[2Jb\r\n"), "a\\u{1b}[2Jb\\r\\n");
    }

    #[test]
    fn expand_home_only_touches_leading_tilde_slash() {
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(
            expand_home("~/x/y.txt"),
            Path::new(&home).join("x/y.txt").display().to_string()
        );
        assert_eq!(expand_home("/tmp/~/z"), "/tmp/~/z");
        assert_eq!(expand_home("~user/z"), "~user/z");
    }

    #[test]
    fn file_header_length_does_not_depend_on_field_values() {
        let short = file_header("tuxi", 0, 0, 1, 0, "a.bin");
        let long = file_header(
            "tuxi",
            u32::MAX,
            u32::MAX - 1,
            u32::MAX,
            MAX_FILE_SIZE,
            "a.bin",
        );
        assert_eq!(short.len(), long.len());
        assert!(file_chunk_capacity("tuxi", &"x".repeat(2000)).is_err());
    }

    #[test]
    fn file_chunk_roundtrip_and_rejections() -> Result<()> {
        let mut frame = file_header("asgard", 7, 2, 3, 5000, "/tmp/a b.bin").into_bytes();
        frame.extend_from_slice(&[9u8; 100]);
        let (from, body) = split_sender(&frame);
        assert_eq!(from, "asgard");
        let chunk = parse_file_chunk(body)?;
        assert_eq!(
            (chunk.id, chunk.index, chunk.count, chunk.size, chunk.path),
            (7, 2, 3, 5000, "/tmp/a b.bin")
        );
        assert_eq!(chunk.data, &[9u8; 100]);

        assert!(parse_file_chunk(b"plain text").is_err());
        assert!(
            parse_file_chunk(b"/file 1 2 3 4 p").is_err(),
            "missing newline"
        );
        let body = |header: String| {
            let frame = header.into_bytes();
            let (_, body) = split_sender(&frame);
            body.to_vec()
        };
        assert!(parse_file_chunk(&body(file_header("a", 1, 0, 1, 1, "p"))).is_ok());
        assert!(parse_file_chunk(&body(file_header("a", 1, 3, 3, 1, "p"))).is_err());
        assert!(parse_file_chunk(&body(file_header("a", 1, 0, 0, 1, "p"))).is_err());
        assert!(
            parse_file_chunk(&body(file_header("a", 1, 0, 1, MAX_FILE_SIZE + 1, "p"))).is_err()
        );
        assert!(parse_file_chunk(&body(file_header("a", 1, 0, 1, 1, "bad\x07name"))).is_err());
        let mut oversized = body(file_header("a", 1, 0, 1, 1, "p"));
        oversized.resize(oversized.len() + psk::MAX_BLOCK_MESSAGE + 1, 0);
        assert!(parse_file_chunk(&oversized).is_err());
        Ok(())
    }

    #[test]
    fn file_send_streams_whole_file_in_single_block_chunks() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("payload.bin");
        let payload: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &payload)?;
        let path = path.display().to_string();
        let mut send = FileSend::open("tuxi", &path)?;
        assert_eq!(send.size, 3000);
        assert_eq!(send.count, 3000u64.div_ceil(send.capacity as u64) as u32);

        let mut incoming = Incoming::new();
        let mut assembled = None;
        let mut sent = 0;
        while let Some(frame) = send.next_chunk("tuxi")? {
            assert_eq!(psk::padded_len(frame.len()), psk::PAD_BLOCK);
            let (from, body) = split_sender(&frame);
            let chunk = parse_file_chunk(body)?;
            let (_, complete) = store_chunk(&mut incoming, &from, &chunk)?;
            if let Some((received_path, data)) = complete {
                assert_eq!(received_path, path);
                assembled = Some(data);
            }
            sent += 1;
        }
        assert_eq!(sent, send.count);
        assert_eq!(assembled.as_deref(), Some(payload.as_slice()));
        assert!(incoming.is_empty());

        let empty = dir.path().join("empty.bin");
        std::fs::write(&empty, b"")?;
        let mut send = FileSend::open("tuxi", &empty.display().to_string())?;
        assert_eq!(send.count, 1);
        assert!(send.next_chunk("tuxi")?.is_some());
        assert!(send.next_chunk("tuxi")?.is_none());
        assert!(FileSend::open("tuxi", &dir.path().display().to_string()).is_err());
        Ok(())
    }

    #[test]
    fn store_chunk_rejects_mismatched_transfer_and_tolerates_reordering() -> Result<()> {
        let mut incoming = Incoming::new();
        let mut frames: Vec<Vec<u8>> = (0..3u32)
            .map(|index| {
                let mut frame = file_header("a", 1, index, 3, 3, "f.bin").into_bytes();
                frame.push(b'0' + index as u8);
                frame
            })
            .collect();
        frames.swap(0, 2);
        let mut result = None;
        for frame in &frames {
            let (from, body) = split_sender(frame);
            let chunk = parse_file_chunk(body)?;
            let (_, complete) = store_chunk(&mut incoming, &from, &chunk)?;
            if complete.is_some() {
                result = complete;
            }
        }
        assert_eq!(result, Some(("f.bin".to_owned(), b"012".to_vec())));

        let first = file_header("a", 2, 0, 2, 10, "f.bin").into_bytes();
        let other = file_header("a", 2, 1, 2, 11, "f.bin").into_bytes();
        store_chunk(
            &mut incoming,
            "a",
            &parse_file_chunk(split_sender(&first).1)?,
        )?;
        assert!(
            store_chunk(
                &mut incoming,
                "a",
                &parse_file_chunk(split_sender(&other).1)?
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn received_file_lands_in_cwd_under_a_free_name_or_reports_the_error() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let cwd = std::env::current_dir()?;
        let dir = tempfile::tempdir()?;
        std::env::set_current_dir(dir.path())?;
        let first = save_received_file("/home/someone/photos/in.tar.gz", b"one");
        let second = save_received_file("/elsewhere/../in.tar.gz", b"two");
        let third = save_received_file("in.tar.gz", b"three");
        let plain = save_received_file("/x/notes", b"four");
        let plain_again = save_received_file("/y/notes", b"five");
        let traversal = save_received_file("/tmp/..", b"six");
        let root = save_received_file("/", b"seven");
        std::env::set_current_dir(&cwd)?;
        assert_eq!(first, Ok(Path::new(".").join("in.tar.gz")));
        assert_eq!(second, Ok(Path::new(".").join("in.tar-2.gz")));
        assert_eq!(third, Ok(Path::new(".").join("in.tar-3.gz")));
        assert_eq!(plain, Ok(Path::new(".").join("notes")));
        assert_eq!(plain_again, Ok(Path::new(".").join("notes-2")));
        assert!(traversal.is_err() && root.is_err());
        assert_eq!(std::fs::read(dir.path().join("in.tar.gz"))?, b"one");
        assert_eq!(std::fs::read(dir.path().join("in.tar-2.gz"))?, b"two");
        assert_eq!(std::fs::read(dir.path().join("in.tar-3.gz"))?, b"three");
        assert_eq!(std::fs::read_dir(dir.path())?.count(), 5);

        let readonly = tempfile::tempdir()?;
        std::fs::set_permissions(readonly.path(), std::fs::Permissions::from_mode(0o555))?;
        if File::create(readonly.path().join("canary")).is_ok() {
            return Ok(()); // privileged user: permissions are not enforced
        }
        std::env::set_current_dir(readonly.path())?;
        let denied = save_received_file("/home/someone/in.txt", b"eight");
        std::env::set_current_dir(cwd)?;
        std::fs::set_permissions(readonly.path(), std::fs::Permissions::from_mode(0o755))?;
        assert!(denied.is_err_and(|err| err.contains("cannot write ./in.txt")));
        Ok(())
    }

    #[test]
    fn unsaved_files_are_shown_as_base64_only_up_to_one_mib() {
        let saved = delivery_report(
            "host1",
            "/x/a.bin",
            b"hello",
            Ok(Path::new(".").join("a.bin")),
        );
        assert_eq!(
            saved,
            ["host1> received-file: /x/a.bin (5 bytes) saved to ./a.bin"]
        );
        let small = delivery_report(
            "host1",
            "/x/a.bin",
            b"hello",
            Err("cannot write".to_owned()),
        );
        assert_eq!(
            small,
            [
                "host1> received-file: /x/a.bin (5 bytes) not saved (cannot write); base64 follows",
                "",
                "aGVsbG8=",
                ""
            ]
        );
        let limit = vec![0u8; MAX_INLINE_FILE];
        let shown = delivery_report("host1", "/x/a.bin", &limit, Err(String::new()));
        assert_eq!(shown.len(), 4);
        assert_eq!(shown[2].len(), MAX_INLINE_FILE.div_ceil(3) * 4);
        let big = vec![0u8; MAX_INLINE_FILE + 1];
        let discarded = delivery_report("host1", "/x/a.bin", &big, Err("cannot write".to_owned()));
        assert_eq!(discarded.len(), 1);
        assert!(
            discarded[0].contains("(1048577 bytes) not saved (cannot write); lost")
                && discarded[0].contains("bigger than 1048576 bytes")
        );
    }

    #[test]
    fn progress_is_reported_every_ten_percent_or_every_interval() {
        let start = Instant::now();
        let mut progress = Progress::at(start);
        let steps: Vec<_> = (1..=100)
            .filter_map(|done| progress.step_at(done, 100, start))
            .collect();
        assert_eq!(steps, [10, 20, 30, 40, 50, 60, 70, 80, 90]);
        let mut progress = Progress::at(start);
        assert_eq!(progress.step_at(1, 3, start), Some(33));
        assert_eq!(progress.step_at(2, 3, start), Some(66));
        assert_eq!(progress.step_at(3, 3, start), None);

        let mut progress = Progress::at(start);
        let almost = start + PROGRESS_INTERVAL - Duration::from_millis(1);
        assert_eq!(progress.step_at(5, 100, almost), None);
        assert_eq!(progress.step_at(5, 100, start + PROGRESS_INTERVAL), Some(5));
        let later = start + 2 * PROGRESS_INTERVAL;
        assert_eq!(
            progress.step_at(6, 100, later - Duration::from_millis(1)),
            None
        );
        assert_eq!(
            progress.step_at(10, 100, later - Duration::from_millis(1)),
            Some(10)
        );
        assert_eq!(progress.step_at(11, 100, later), None);
        assert_eq!(
            progress.step_at(11, 100, later + PROGRESS_INTERVAL),
            Some(11)
        );
        assert_eq!(
            progress.step_at(100, 100, later + 3 * PROGRESS_INTERVAL),
            None
        );
    }

    #[test]
    fn received_messages_produce_lines_only_when_there_is_something_to_show() -> Result<()> {
        let mut incoming = Incoming::new();
        assert_eq!(
            receive_lines(&mut incoming, &frame("bob", "hi")),
            ["bob> hi"]
        );
        let mut first = file_header("bob", 7, 0, 20, 20, "f.bin").into_bytes();
        first.push(b'x');
        assert!(receive_lines(&mut incoming, &first).is_empty());
        let mut second = file_header("bob", 7, 1, 20, 20, "f.bin").into_bytes();
        second.push(b'y');
        assert_eq!(
            receive_lines(&mut incoming, &second),
            ["[bob> file: 10% received]"]
        );
        assert!(receive_lines(&mut incoming, b"bob\n/file broken").is_empty());
        Ok(())
    }
}
