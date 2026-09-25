# nym-pq-chat: post-quantum-safe end-to-end encryption on top of the Nym mixnet

`tools/nym-pq-chat` is a small CLI that sends chat lines through the Nym mixnet after
encrypting them with a **pre-shared 256-bit key** (`key.secret`) that both machines hold locally
and that must be moved between them offline (USB stick), never over a network. Everything that leaves the tool
towards the mixnet is ciphertext; everything received is authenticated and decrypted with the same key.

The two machines are called **host1** and **host2** below (the reference run was done with two FreeBSD 15.1
laptops on one home WiFi, each talking to a different Nym gateway; a second run paired a FreeBSD laptop with an
Ubuntu Linux machine, so the two ends may run different operating systems).

## Threat model

| | |
|---|---|
| Protected | Message content and integrity, against anyone recording the traffic today and decrypting later with a quantum computer ("harvest now, decrypt later"), including all Nym nodes. |
| Not protected | Metadata: an observer of your uplink sees that *your* IP talks to *a Nym gateway* (plus Nym API over HTTPS and DNS-over-HTTPS/TLS resolvers). It does not learn the peer or the content. The mixnet hides who talks to whom from the network itself. |
| Not provided | Forward secrecy: if `key.secret` leaks, recorded messages encrypted with it become readable. Rotate the key (rerun `keygen`, copy again offline). |

## Cryptography

* Key file `key.secret`: 64 random bytes, hex encoded (`keygen`, 128 hex characters). Any file with >= 64 bytes
  works, but only random bytes give the full 256-bit strength.
* Key derivation: `HKDF-SHA-512(salt="nym-pq-chat/psk/v1", ikm=key.secret)` -> 256-bit message key
  (+ a separate 4-byte *fingerprint* shown at startup, so you can check both machines have the same key
  without comparing the key itself).
* Message encryption: **XChaCha20-Poly1305** AEAD, fresh random 192-bit nonce per message.
  Wire format: `0x02 || nonce(24) || ciphertext || tag(16)` (41 bytes overhead). Before encryption every
  message is prefixed with its 4-byte length and zero-padded to a multiple of 1024 bytes, so the ciphertext
  length only reveals the size class: `hi` and a 1000-character line are both 1065 bytes, and every chunk of
  a file transfer is the same 1065 bytes. Messages that fail authentication (wrong key, tampering, random
  traffic) are dropped with a warning; control characters in received text or file names are escaped before
  printing, so a peer cannot drive your terminal.
* Why this is post-quantum safe: the layer is purely symmetric. There is no key exchange or signature for
  Shor's algorithm to break; Grover's algorithm reduces a 256-bit key to ~2^128 quantum work, which is
  NIST PQC security level 5 (same as AES-256). This is the same cipher family OpenSSH uses for its transport
  (`chacha20-poly1305@openssh.com`); SSH's quantum problem is its key exchange, which a pre-shared key does
  not need. SHA-512 is the hash OpenSSH uses in its PQ hybrid `sntrup761x25519-sha512`.
* Below this layer Nym adds its own encryption (Sphinx onion layers between hops, a client<->gateway shared key,
  optionally TLS to the gateway). Those use X25519 and are *not* quantum-safe, which is exactly why this layer exists.

## Build (on every machine)

Rust >= 1.87; tested with rustc 1.97.1 and 1.98.1 on FreeBSD 15.1 (16 cores: a cold release build takes
~4 minutes, an incremental one ~15 s) and with rustc 1.98.1 on Ubuntu Linux (cold release build ~2 minutes on
16 cores). Nothing in the tool is OS specific; FreeBSD and Linux builds interoperate (see the second reference run).

```sh
cd </path/to/nym>                                        # repository root, branch unicron-add-communication-encryption
cargo build --release -p nym-pq-chat
cargo test  --release -p nym-pq-chat                     # 11 unit tests: roundtrip, wrong key, tampering, key file handling
mkdir -p ~/bin && install -m 755 target/release/nym-pq-chat ~/bin/   # or anywhere on PATH
nym-pq-chat --version
```

Only this one binary (~25 MB) is needed at runtime. If both machines have the same OS/architecture you can
copy `target/release/nym-pq-chat` instead of building twice.

Cleanup: everything else under `target/` is intermediate build output (1.4-2.6 GB, ~10-20k files). Once the
binary is installed, remove it; the repository shrinks to its ~200 MB of source:

```sh
cargo clean                                              # deletes target/ entirely, so install the binary first
```

The next `cargo build` is a full cold build again. Downloaded crate sources live in `~/.cargo/registry`
(0.6-1.4 GB, shared by every Rust project on the machine); `cargo clean` leaves them alone and they are only
needed to build again.

Getting the source onto the second machine without going through GitHub (`target/` is excluded because it is
build output that gets rebuilt anyway):

```sh
# on host1
rsync -a --delete --exclude target/ --exclude '*.secret' --exclude nym-pq-chat-storage/ \
  </path/to/nym>/ host2:</path/to/nym>/
```

Once the branch is pushed, `git clone` + `git checkout unicron-add-communication-encryption` does the same.

## Files and command line

All state lives in one directory (`--dir`, use the same one for every command). Machine names are given with
`--me` / `--peer` (or `--peers`); they only decide file names, the `name>` prefix of received lines and the
`name: ` input prompt.

| file in `~/pqchat` | created by | copy to the other machine? |
|---|---|---|
| `key.secret` | `keygen` (mode 0600) | **yes, offline only** (USB stick), identical on both |
| `host1.address.secret` | `--me host1 init` on host1 | yes, same file name, into host2's `~/pqchat` |
| `host2.address.secret` | `--me host2 init` on host2 | yes, same file name, into host1's `~/pqchat` |
| `nym-pq-chat-storage/` | `init` / `run` | **never** - this is the machine's own Nym identity (private keys) |

After setup both directories contain the same three `*.secret` files. All of them are `.gitignore`d
(`*.secret`, `nym-pq-chat-storage/`). The `.address.secret` files are not really secret (a Nym address is
public information for whoever you give it to) but they identify you, so they are kept 0600 like the key.

```
nym-pq-chat [--dir DIR] [--me NAME] [--peer|--peers NAME[,NAME...]] [--tls] <command>
  keygen    write a fresh random key.secret
  init      create/load this machine's mixnet identity, save its address to <me>.address.secret
  run       chat (stdin -> encrypt -> every peer; peer -> decrypt -> stdout); --show-ciphertext prints hex of every message
  encrypt   stdin -> hex ciphertext (offline test)
  decrypt   hex ciphertext -> stdout (offline test)
```

Like e-mail: one sender identity (`--me`), any number of recipients (`--peer host2,host3`, `--peers host2,host3`
or repeated `--peer`; both `--peer` and `--peers` are accepted everywhere). Every line is encrypted once and sent
to each listed address; list yourself to get your own lines echoed back through the mixnet. The sender's name
travels inside the encrypted payload (`<me>\n<line>`) and is printed as the `name>` prefix, so a receiver with
several peers knows who wrote (any key holder can claim any name). In an interactive terminal the input prompt is
your own name (`host1: `), so you always know which identity you are typing as.

`--tls` restricts the client to gateways that offer `wss://` (port 9001) so the last plaintext-ish thing an
observer sees (the websocket HTTP upgrade with the gateway host name) disappears too. The gateway is chosen at
the first `init`/`run` and persisted in `nym-pq-chat-storage/`, so decide before `init`; `--tls` on a directory
whose stored gateway was registered without TLS is refused with an error instead of silently using `ws://`
(delete the storage directory and re-run `init --tls`, which gives you a new address to re-exchange).

`run` also works non-interactively (`echo "text" | nym-pq-chat ... run`): after stdin is closed it stays connected
until a running file transfer is finished and for 5 s after the last sent line so queued messages reach the
gateway, then disconnects. Only one `run`/`init` per identity at a time: the gateway refuses a second connection
(`There is already an open connection to this client`), so stop the running instance first (`pgrep -fl nym-pq-chat`).

### Text lines and files

A chat line may be up to 4096 bytes (longer lines are refused with a hint, they are meant to be short). A line of
up to 1020 bytes including your name and a newline (`tuxi\n` + 1015 bytes) is one 1065-byte ciphertext = one mixnet
packet; longer ones are padded to the next KiB and the Nym client splits them into several packets and reassembles
them on the other side, the receiver just sees the line.
Terminals cap what you can type or paste on one line before the tool sees it: 4095 bytes on Linux, 1919 bytes on
FreeBSD (a longer paste never completes the line; press Ctrl-U to clear it) - piped input has no such cap.

`/file: <path>` (leading `~/` allowed) sends that file to every peer, up to 50 MiB. The file is
streamed as chunks of just under 1 KiB (header + data = 1020 bytes, e.g. 940 data bytes when `host1` sends
`/home/me/photos/cat.jpg`), each encrypted into the same 1065-byte message as a chat line, so file
chunks and chat lines are indistinguishable on the wire; chunks go through a separate queue so chat lines typed
meanwhile are not delayed. Files are sent one after another: a `/file:` typed while another file is going out is
queued (`[queued file <path> (<size> bytes) behind N other file(s)]`) and starts by itself when the previous one is
done. Receiving is asynchronous: files from several peers (at most one per peer, since every sender is sequential)
are reassembled independently and each is saved as soon as its last chunk arrives. Progress is printed on both sides
every 10 % or every 30 s, whichever comes first (`[file <path>: 40% sent]`, `[host1> file <path>: 40% received]`).
The receiver prints
`host1> received-file: <path> (<size> bytes) saved to ./<file name>`: the sender's path is only displayed, the
file is created as `./<file name>` in the directory the receiver runs in, never overwriting anything: if that name
is taken it becomes `./<name>-2.<ext>`, `./<name>-3.<ext>`, ... (first free one); if the file cannot be written at
all (e.g. read-only directory), a file up to 1 MiB is printed as base64 between two empty lines (about 1.33 MiB of
text) and a bigger one is discarded with a message saying so. Speed is bounded by the mixnet client
(about 35-50 packets/s, roughly 35-50 KB/s: 5 MiB took 2.5 min between two laptops); the receiving side keeps the
chunks in memory until the last one arrives and there is no resume if either side stops midway (Ctrl-C during a
transfer aborts it; an incomplete incoming file that gets no chunk for 10 minutes is dropped with a message).
Chunks already inside the mixnet when a sender quits are still delivered - even to a receiver that restarts, since
the gateways buffer them - so a fresh client may print a `0%`/`1%` progress line for such a leftover transfer, which
is then dropped the same way. Only the remote's `key.secret` holder can send you files, and all it can ever do is
*create* a new file named after the sent one in your current directory.

## Full example: host1 <-> host2

Step 1, key (on host1 only):

```sh
host1$ mkdir -p ~/pqchat
host1$ nym-pq-chat --dir ~/pqchat keygen
wrote ~/pqchat/key.secret (key fingerprint 16a9c31b)
copy it to the peer machine OFFLINE (USB stick) - never send it over any network
```

Copy `~/pqchat/key.secret` to a USB stick, carry it to host2, put it in host2's `~/pqchat/`, wipe the stick.
(The reference runs below took a shortcut: both laptops were on the same home LAN and
`scp -p ~/pqchat/key.secret host2:~/pqchat/` was used. That is not the offline procedure the claims above rely
on - the key crossed the LAN inside an SSH session - so those runs demonstrate the chat layer, not the key
transfer.) Check it is identical: `sha256 ~/pqchat/key.secret` on both (`sha256sum` on Linux). Keys made by
older builds (64 hex characters) still load; `keygen` now writes 128 - to rotate, rerun it and redistribute.

Step 2, identities (on both; the first connection registers with a gateway and takes 10-20 s):

```sh
host1$ nym-pq-chat --dir ~/pqchat --me host1 init
our (host1) nym address: 2EnCLyEg...9AGw.3E3UbqAq...KBrv@9Lf7mj1iHMiV72anBpJEkEaTkXLKJeHC9Ex9fV5ZJu81
saved to ~/pqchat/host1.address.secret; copy it (same file name) into the peer machine's chat directory

host2$ nym-pq-chat --dir ~/pqchat --me host2 init
our (host2) nym address: 56dVSEQv...qGDZ.FY3tf3g4...chA5@AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR
saved to ~/pqchat/host2.address.secret; copy it (same file name) into the peer machine's chat directory
```

(Addresses abbreviated here; the real ones are three full base58 strings, `<client-identity>.<client-encryption>@<gateway-identity>`.)

Step 3, exchange the address files (USB stick, or on the LAN):

```sh
host1$ scp -p host2:~/pqchat/host2.address.secret ~/pqchat/
host1$ scp -p ~/pqchat/host1.address.secret host2:~/pqchat/
```

Both machines now have:

```
~/pqchat/key.secret               -rw-------  129 bytes, identical
~/pqchat/host1.address.secret     -rw-------  135 bytes, identical
~/pqchat/host2.address.secret     -rw-------  135 bytes, identical
~/pqchat/nym-pq-chat-storage/     private, different on each machine
```

Step 4, chat (both machines; each one uses its own name as `--me` and the other as `--peer`):

```sh
host1$ nym-pq-chat --dir ~/pqchat --me host1 --peer host2 run
host2$ nym-pq-chat --dir ~/pqchat --me host2 --peer host1 run
```

Both print their own address, then (the last line is the input prompt: your own name):

```
pre-shared key fingerprint: 16a9c31b (must be identical on the peers)
peer host2: 56dVSEQv...qGDZ.FY3tf3g4...chA5@AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR
type a line and press Enter to send it; /file: <path> sends a file; Ctrl-D or Ctrl-C quits
host1: 
```

Check the fingerprint is the same on both. Type a line and press Enter; a few seconds later it shows up on the
other machine prefixed with the sender's name. Test transcript (with `--show-ciphertext`), on host1:

```
host1: hello host2, this is host1 MARKER-HOST1-1
[sending 1065 bytes of ciphertext: 027bbc5de7acb741f9f0dcf3f315aa85d334220daad8...]
host1: 
[received 1065 bytes of ciphertext: 0286bc1bcc0f146dc39f35ae2aa26eea7c21eb7fdc4c...]
host2> hello host1, this is host2 MARKER-HOST2-1
host1: /file: ~/photos/cat.jpg
[sending file /home/me/photos/cat.jpg (2718091 bytes) as 2892 messages of 1065 bytes]
host1: 
[file /home/me/photos/cat.jpg: 10% sent]
host1: 
...
[file /home/me/photos/cat.jpg (2718091 bytes) sent as 2892 messages]
host1: 
```

and on host2 (output is only ever appended, like in a shell: anything that arrives asynchronously - a peer's line, a
progress report, a delivered file - is printed as a block starting on a new line below the prompt line, followed by a
fresh prompt; the text you had typed but not yet sent is not shown again, but it is still in the terminal's input
buffer: keep typing, Enter sends it all as one line):

```
host2: 
[received 1065 bytes of ciphertext: 027bbc5de7acb741f9f0dcf3f315aa85d334220daad8...]
host1> hello host2, this is host1 MARKER-HOST1-1
host2: hello host1, this is host2 MARKER-HOST2-1
[sending 1065 bytes of ciphertext: 0286bc1bcc0f146dc39f35ae2aa26eea7c21eb7fdc4c...]
host2: 
[host1> file /home/me/photos/cat.jpg: 10% received]
host2: 
...
host1> received-file: /home/me/photos/cat.jpg (2718091 bytes) saved to ./cat.jpg
host2: 
```

(`/home/me/photos/` does not exist on host2, so the file landed in host2's current directory.) Messages are
independent and may arrive in a different order than sent; the peer can be offline: the gateway stores messages
and delivers them when its client reconnects.
Logs go to stderr (`RUST_LOG=info` for the mixnet client's own logs), chat to stdout, so `run` also works
non-interactively (`tail -f in.txt | nym-pq-chat ... run > out.txt` is how the test below was driven over ssh);
the prompt is only shown when stdin and stdout are a terminal.

Quick single-machine test: `--me host1 --peer host1 run` - you receive your own lines back through the mixnet as
`host1> ...`.

More than two machines: every machine gets every other machine's `<name>.address.secret` and runs with all of
them, e.g. `host1$ nym-pq-chat --dir ~/pqchat --me host1 --peer host2,host3 run` (add `host1` to the list to see
your own lines come back too); each received line is prefixed with the name of whoever sent it.

## Proving it is safe

### 1. The encryption layer alone (offline, no network)

```sh
# on host1
echo "meet at 18:00" | nym-pq-chat --dir ~/pqchat encrypt > msg.hex      # hex ciphertext, carry via USB
# on host2 (same key.secret)
nym-pq-chat --dir ~/pqchat decrypt < msg.hex                              # -> meet at 18:00
# on any machine with a different key.secret
nym-pq-chat --dir /tmp/otherkey decrypt < msg.hex                         # -> Error: authentication failed
```

Flip any hex digit in `msg.hex` and `decrypt` fails too (Poly1305 tag). Encrypting the same text twice gives
different ciphertext (random nonce).

### 2. A wrong key is rejected on the live network

A third client with its own `key.secret` (fingerprint `30007a1e`) and host2's address as `--peer` sent
`intruder with wrong key MARKER-INTRUDER`. It was delivered by the mixnet (80 bytes of ciphertext arrived) and
host2 dropped it:

```
WARN tools/nym-pq-chat/src/main.rs:233: dropping 80 byte message: authentication failed: not encrypted with our key.secret
```

Nothing is printed as chat, nothing is sent back. Knowing an address lets you send noise, not read or forge.

### 3. What actually travels over the network

Run the chat with `--show-ciphertext` so you know the exact bytes the layer produced, capture on your uplink
interface on **both** laptops while chatting, then search the captures.

FreeBSD (interface of the default route: `netstat -rn -f inet | grep default`, `wlan0` below):

```sh
sudo tcpdump -i wlan0 -nn -w </path/to>/chat.pcap 'not port 22'      # Ctrl-C when done chatting
```

Linux equivalent: `sudo tcpdump -i wlan0 -nn -w chat.pcap 'not port 22'` (interface from `ip route | head -1`).
Wireshark opens the same `.pcap`.

While `run` is connected, list the remote hosts the tool talks to:

```sh
sockstat -4 -c | grep nym-pq                 # FreeBSD
ss -tnp | grep nym-pq-chat                   # Linux
```

Output on host1 during the test (`<LAN-IP>` = host1's own address):

```
user nym-pq-cha 67047 21 tcp4  <LAN-IP>:24837   92.39.63.14:443        <- Nym API (network topology)
user nym-pq-cha 67047 26 tcp4  <LAN-IP>:32019   149.112.112.112:443    <- DNS over HTTPS (Quad9)
user nym-pq-cha 67047 17 tcp4  <LAN-IP>:19177   1.0.0.1:443            <- DNS over HTTPS (Cloudflare)
user nym-pq-cha 67047 27 tcp4  <LAN-IP>:45997   213.218.160.12:9000    <- the gateway (ws://; :9001 with --tls)
```

The gateway of a machine is the part after `@` in its address; map identity -> IP with
`curl -s 'https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true' | jq '.nodes.data[] | select(.ed25519_identity_pubkey=="<identity>") | {ip_addresses, entry}'`.
In the test: host1 -> `213.218.160.12:9000`, host2 -> `49.12.42.50:9000`.

Analyse a capture (`GW` = that machine's gateway IP, `LAN` = your local subnet):

```sh
P=</path/to>/chat.pcap; GW=<GATEWAY_IP>; LAN=<LAN>/24

# 1. where did this machine send packets? (only gateway, Nym API, DNS resolvers, NTP, LAN broadcast)
tcpdump -nn -q -r $P "src net $LAN and not dst net $LAN and not dst net 224.0.0.0/4" \
  | awk '{print $5}' | sed 's/:$//' | sort | uniq -c | sort -rn | head

# 2. plaintext never appears (use words you actually typed)
grep -a -c "MARKER-HOST1-1" $P                                           # -> 0

# 3. even the PSK ciphertext printed by --show-ciphertext never appears as-is:
#    it is wrapped in Sphinx packets and the client<->gateway encryption
python3 -c 'import sys;c=open(sys.argv[1],"rb").read();ct=bytes.fromhex(sys.argv[2]);print(c.count(ct))' $P <hex-from-show-ciphertext>   # -> 0

# 4. only fixed-size packets at a constant rate towards the gateway (cover traffic), message timing is hidden
tcpdump -nn -r $P "dst host $GW and greater 200" | grep -oE "length [0-9]+" | sort | uniq -c | sort -rn | head
tcpdump -nn -tt -r $P "dst host $GW and greater 200" | awk '{t=int($1/10)*10;c[t]++} END{for(k in c)print k,c[k]}' | sort -n

# 5. everything the observer gets: your IP <-> gateway IP, byte counts, timing
tcpdump -nn -q -r $P "host $GW" | awk '{print $3" -> "$5}' | sed 's/:$//' | sort | uniq -c

# 6. the only readable text in the gateway stream without --tls: the websocket upgrade
tcpdump -nn -A -r $P "host $GW and port 9000" | grep -aE "^(GET|Host:|Upgrade:)"
```

Wireshark filter for the same view: `ip.addr == <GATEWAY_IP>`; with `--tls` the stream is TLS, without it
`Follow TCP stream` shows the HTTP websocket upgrade followed by binary frames that are already ciphertext.

### Reference run 1: two FreeBSD laptops, 2026-09-25

Two laptops (host1, host2), two gateways, tcpdump on `wlan0` of each, 184 s session, 4 chat lines (2 each way,
80-100 bytes of ciphertext each) plus the wrong-key intruder message:

| | host1 capture | host2 capture |
|---|---|---|
| packets captured | 40 498 | 124 861 (host also runs other things) |
| packets to / from own gateway | 20 585 / 19 270 | 21 414 / 20 372 |
| hits for the 8 plaintext markers (`MARKER-*`, `secret meeting`, `hello ...`) | 0 | 0 |
| hits for the 5 exact ciphertexts printed by `--show-ciphertext` | 0 | 0 |
| rate to gateway | ~100 packets/s, constant, from connect to disconnect | ~100 packets/s, constant |
| segment sizes to gateway (most to least frequent) | 1428, 1032, 636, 240, 1272 | 1428, 1032, 636, 240, 1272 |
| other remote endpoints | `92.39.63.14:443` Nym API, `1.0.0.1` / `149.112.112.112` / `9.9.9.9` `:443`/`:853` DNS, `:123` NTP | same kind |
| readable text in gateway stream | `Host: nym-exit.ro-2.silentriver.foo:9000`, `Upgrade: websocket` | `Host: nymgw1.tinkerbase.org:9000`, `Upgrade: websocket` |

The message rate did not change when lines were typed: the client sends cover packets at the same rate whether
or not there is anything to say. Strings extracted from the gateway stream (`strings -n 8`) are random bytes.

### Reference run 2: FreeBSD <-> Linux, 2026-09-25

host1 = FreeBSD 15.1 laptop, host2 = Ubuntu Linux (a VM NATed through host1's WiFi, so host1's capture also
contains host2's traffic), two gateways, tcpdump on `wlan0` (FreeBSD) and `enp0s6` (Linux). Three lines: one
each way from interactive `run`, one from Linux via `echo "... MARKER" | nym-pq-chat ... run` (stdin closed
immediately; the line was still delivered and the client exited after ~6 s). Every ciphertext printed by the sender
was printed byte-for-byte by the receiver, and both decrypted to the typed text; key fingerprint `65c278f7` on both.

| | FreeBSD capture | Linux capture |
|---|---|---|
| packets captured | 70 244 (incl. the VM's) | 28 524 |
| packets to own gateway (`49.12.42.50` / `185.100.84.193`) | 20 555 | 10 660 |
| hits for the 6 plaintext markers | 0 | 0 |
| hits for the 3 exact ciphertexts | 0 | 0 |
| rate / segment sizes to gateway | ~100 packets/s constant; 1428, 1032, 636, 240, 1272 | same |
| other remote endpoints | Nym API `92.39.63.14:443`, DNS resolvers `:443`/`:853` | same kind |

### 4. Trust boundaries

* The peer address (`<peer>.address.secret`) tells the mixnet where to deliver; a wrong or spoofed address only
  causes undecryptable messages on the other side, never plaintext exposure.
* Anyone who obtains `key.secret` can read and forge messages. Keep it 0600, move it only offline, rotate it.
* Deleting `nym-pq-chat-storage/` gives the machine a new Nym identity/address (re-run `init` and re-exchange
  the `.address.secret` file).
