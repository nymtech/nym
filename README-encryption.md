# nym-pq-chat: post-quantum-safe end-to-end encryption on top of the Nym mixnet

`tools/nym-pq-chat` is a small CLI that sends chat lines through the Nym mixnet after
encrypting them with a **pre-shared 256-bit key** (`key.secret`) that both machines hold locally
and that never touches any network. Everything that leaves the tool towards the mixnet is
ciphertext; everything received is authenticated and decrypted with the same key.

This document uses the two real machines it was tested on: **tuxi** and **asgard** (both FreeBSD 15.1
laptops on one home WiFi, each talking to a different Nym gateway).

## Threat model

| | |
|---|---|
| Protected | Message content and integrity, against anyone recording the traffic today and decrypting later with a quantum computer ("harvest now, decrypt later"), including all Nym nodes. |
| Not protected | Metadata: an observer of your uplink sees that *your* IP talks to *a Nym gateway* (plus Nym API over HTTPS and DNS-over-HTTPS/TLS resolvers). It does not learn the peer or the content. The mixnet hides who talks to whom from the network itself. |
| Not provided | Forward secrecy: if `key.secret` leaks, recorded messages encrypted with it become readable. Rotate the key (rerun `keygen`, copy again offline). |

## Cryptography

* Key file `key.secret`: 32 random bytes, hex encoded (`keygen`). Any file with >= 32 bytes works,
  but only random bytes give the full 256-bit strength.
* Key derivation: `HKDF-SHA-512(salt="nym-pq-chat/psk/v1", ikm=key.secret)` -> 256-bit message key
  (+ a separate 4-byte *fingerprint* shown at startup, so you can check both machines have the same key
  without comparing the key itself).
* Message encryption: **XChaCha20-Poly1305** AEAD, fresh random 192-bit nonce per message.
  Wire format: `0x01 || nonce(24) || ciphertext || tag(16)` (41 bytes overhead). Messages that fail
  authentication (wrong key, tampering, random traffic) are dropped with a warning.
* Why this is post-quantum safe: the layer is purely symmetric. There is no key exchange or signature for
  Shor's algorithm to break; Grover's algorithm reduces a 256-bit key to ~2^128 quantum work, which is
  NIST PQC security level 5 (same as AES-256). This is the same cipher family OpenSSH uses for its transport
  (`chacha20-poly1305@openssh.com`); SSH's quantum problem is its key exchange, which a pre-shared key does
  not need. SHA-512 is the hash OpenSSH uses in its PQ hybrid `sntrup761x25519-sha512`.
* Below this layer Nym adds its own encryption (Sphinx onion layers between hops, a client<->gateway shared key,
  optionally TLS to the gateway). Those use X25519 and are *not* quantum-safe, which is exactly why this layer exists.

## Build (on every machine)

Rust >= 1.87; tested with rustc 1.97.1 (tuxi) and 1.98.1 (asgard) on FreeBSD 15.1, 16 cores: a cold
release build takes ~4 minutes, an incremental one ~15 s.

```sh
cd /data/dev/dev2/go/src/github.com/nymtech/nym          # repository root, branch unicron-add-communication-encryption
cargo build --release -p nym-pq-chat
cargo test  --release -p nym-pq-chat                     # 11 unit tests: roundtrip, wrong key, tampering, key file handling
mkdir -p ~/bin && install -m 755 target/release/nym-pq-chat ~/bin/   # ~/bin is on PATH on both laptops
nym-pq-chat --version
```

Only this one binary is needed at runtime. If both machines have the same OS/architecture you can
copy `target/release/nym-pq-chat` instead of building twice.

Getting the source onto the second machine while the branch is not pushed yet (what was done for asgard;
`target/` is excluded because it is 1.4 GB of build output that gets rebuilt anyway):

```sh
# on tuxi
rsync -a --delete --exclude target/ --exclude '*.secret' --exclude nym-pq-chat-storage/ \
  /data/dev/dev2/go/src/github.com/nymtech/nym/ asgard:/data/dev/dev2/go/src/github.com/nymtech/nym/
```

Once the branch is pushed, `git clone` + `git checkout unicron-add-communication-encryption` does the same.

## Files and command line

All state lives in one directory (`--dir`, use the same one for every command). Machine names are given with
`--me` / `--peer`; they only decide file names and the `name>` prefix of received lines.

| file in `~/pqchat` | created by | copy to the other machine? |
|---|---|---|
| `key.secret` | `keygen` (mode 0600) | **yes, offline only** (USB stick), identical on both |
| `tuxi.address.secret` | `--me tuxi init` on tuxi | yes, same file name, into asgard's `~/pqchat` |
| `asgard.address.secret` | `--me asgard init` on asgard | yes, same file name, into tuxi's `~/pqchat` |
| `nym-pq-chat-storage/` | `init` / `run` | **never** - this is the machine's own Nym identity (private keys) |

After setup both directories contain the same three `*.secret` files. All of them are `.gitignore`d
(`*.secret`, `nym-pq-chat-storage/`). The `.address.secret` files are not really secret (a Nym address is
public information for whoever you give it to) but they identify you, so they are kept 0600 like the key.

```
nym-pq-chat [--dir DIR] [--me NAME] [--peer NAME] [--tls] <command>
  keygen    write a fresh random key.secret
  init      create/load this machine's mixnet identity, save its address to <me>.address.secret
  run       chat (stdin -> encrypt -> peer; peer -> decrypt -> stdout); --show-ciphertext prints hex of every message
  encrypt   stdin -> hex ciphertext (offline test)
  decrypt   hex ciphertext -> stdout (offline test)
```

`--tls` restricts the client to gateways that offer `wss://` (port 9001) so the last plaintext-ish thing an
observer sees (the websocket HTTP upgrade with the gateway host name) disappears too. The gateway is chosen at
the first `init`/`run` and persisted in `nym-pq-chat-storage/`, so decide before `init` (or delete the storage
directory and re-run `init`, which gives you a new address to re-exchange).

## Full example: tuxi <-> asgard

Step 1, key (on tuxi only):

```sh
tuxi$ mkdir -p ~/pqchat
tuxi$ nym-pq-chat --dir ~/pqchat keygen
wrote /home/lgryglicki/pqchat/key.secret (key fingerprint 65c278f7)
copy it to the peer machine OFFLINE (USB stick) - never send it over any network
```

Copy `~/pqchat/key.secret` to a USB stick, carry it to asgard, put it in asgard's `~/pqchat/`, wipe the stick.
(For the test both laptops were on the same home LAN and `scp -p ~/pqchat/key.secret asgard:~/pqchat/` was used
instead; the key still never crossed the internet.) Check it is identical: `sha256 ~/pqchat/key.secret` on both.

Step 2, identities (on both; the first connection registers with a gateway and takes 10-20 s):

```sh
tuxi$   nym-pq-chat --dir ~/pqchat --me tuxi init
our (tuxi) nym address: 56dVSEQv1hDm9iCgzpc8SjcMWT19W7DCm4wDEvK6qGDZ.FY3tf3g48iFhrFUyqxcxrZpgszDZQ27s4YqAridkchA5@AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR
saved to /home/lgryglicki/pqchat/tuxi.address.secret; copy it (same file name) into the peer machine's chat directory

asgard$ nym-pq-chat --dir ~/pqchat --me asgard init
our (asgard) nym address: 2EnCLyEgHEq6syZRDVCcHwJKacvDFyBuG1PzC7Pg9AGw.3E3UbqAqzGXEF72vykYSm4RPoxieRFKQSekhboXzKBrv@9Lf7mj1iHMiV72anBpJEkEaTkXLKJeHC9Ex9fV5ZJu81
saved to /home/lgryglicki/pqchat/asgard.address.secret; copy it (same file name) into the peer machine's chat directory
```

Step 3, exchange the address files (USB stick, or on the LAN):

```sh
tuxi$ scp -p asgard:~/pqchat/asgard.address.secret ~/pqchat/
tuxi$ scp -p ~/pqchat/tuxi.address.secret asgard:~/pqchat/
```

Both machines now have:

```
~/pqchat/key.secret               -rw-------   65 bytes, identical
~/pqchat/tuxi.address.secret      -rw-------  135 bytes, identical
~/pqchat/asgard.address.secret    -rw-------  135 bytes, identical
~/pqchat/nym-pq-chat-storage/     private, different on each machine
```

Step 4, chat (both machines; each one uses its own name as `--me` and the other as `--peer`):

```sh
tuxi$   nym-pq-chat --dir ~/pqchat --me tuxi   --peer asgard run
asgard$ nym-pq-chat --dir ~/pqchat --me asgard --peer tuxi   run
```

Both print their own address, then:

```
pre-shared key fingerprint: 65c278f7 (must be identical on the peer)
peer asgard: 2EnCLyEgHEq6syZRDVCcHwJKacvDFyBuG1PzC7Pg9AGw....@9Lf7mj1iHMiV72anBpJEkEaTkXLKJeHC9Ex9fV5ZJu81
type a line and press Enter to send it; Ctrl-D or Ctrl-C quits
```

Check the fingerprint is the same on both. Type a line and press Enter; a few seconds later it shows up on the
other machine prefixed with the sender's name. Test transcript (with `--show-ciphertext`):

```
tuxi$ hello asgard, this is tuxi MARKER-TUXI-1
[sending 81 bytes of ciphertext: 018ece418a747d40abcdc3d5bd151afac870528c1e64...]
[received 83 bytes of ciphertext: 016da35af0ac99e1ca45c865e7818ee9234ede0972e5...]
asgard> hello tuxi, this is asgard MARKER-ASGARD-1

asgard$ [received 81 bytes of ciphertext: 018ece418a747d40abcdc3d5bd151afac870528c1e64...]
tuxi> hello asgard, this is tuxi MARKER-TUXI-1
```

The peer can be offline: the gateway stores messages and delivers them when its client reconnects.
Logs go to stderr (`RUST_LOG=info` for the mixnet client's own logs), chat to stdout, so `run` also works
non-interactively (`tail -f in.txt | nym-pq-chat ... run > out.txt` is how the test below was driven over ssh).

Quick single-machine test: `cp tuxi.address.secret loop.address.secret` and `--me tuxi --peer loop run` - you
receive your own lines back through the mixnet as `loop> ...`.

## Proving it is safe

### 1. The encryption layer alone (offline, no network)

```sh
# on tuxi
echo "meet at 18:00" | nym-pq-chat --dir ~/pqchat encrypt > msg.hex      # hex ciphertext, carry via USB
# on asgard (same key.secret)
nym-pq-chat --dir ~/pqchat decrypt < msg.hex                              # -> meet at 18:00
# on any machine with a different key.secret
nym-pq-chat --dir /tmp/otherkey decrypt < msg.hex                         # -> Error: authentication failed
```

Flip any hex digit in `msg.hex` and `decrypt` fails too (Poly1305 tag). Encrypting the same text twice gives
different ciphertext (random nonce).

### 2. A wrong key is rejected on the live network

A third client with its own `key.secret` (fingerprint `30007a1e`) and tuxi's address as `--peer` sent
`intruder with wrong key MARKER-INTRUDER`. It was delivered by the mixnet (80 bytes of ciphertext arrived) and
tuxi dropped it:

```
WARN tools/nym-pq-chat/src/main.rs:233: dropping 80 byte message: authentication failed: not encrypted with our key.secret
```

Nothing is printed as chat, nothing is sent back. Knowing an address lets you send noise, not read or forge.

### 3. What actually travels over the network

Run the chat with `--show-ciphertext` so you know the exact bytes the layer produced, capture on your uplink
interface on **both** laptops while chatting, then search the captures.

FreeBSD (both laptops route via `wlan0`; check with `netstat -rn -f inet | grep default`):

```sh
sudo tcpdump -i wlan0 -nn -w /data/tmp/chat.pcap 'not port 22'      # Ctrl-C when done chatting
```

Linux equivalent: `sudo tcpdump -i wlan0 -nn -w chat.pcap 'not port 22'` (interface from `ip route | head -1`).
Wireshark opens the same `.pcap`.

While `run` is connected, list the remote hosts the tool talks to:

```sh
sockstat -4 -c | grep nym-pq                 # FreeBSD
ss -tnp | grep nym-pq-chat                   # Linux
```

Real output on asgard during the test:

```
lgryglicki nym-pq-cha 67047 21 tcp4  192.168.1.163:24837   92.39.63.14:443        <- Nym API (network topology)
lgryglicki nym-pq-cha 67047 26 tcp4  192.168.1.163:32019   149.112.112.112:443    <- DNS over HTTPS (Quad9)
lgryglicki nym-pq-cha 67047 17 tcp4  192.168.1.163:19177   1.0.0.1:443            <- DNS over HTTPS (Cloudflare)
lgryglicki nym-pq-cha 67047 27 tcp4  192.168.1.163:45997   213.218.160.12:9000    <- the gateway (ws://; :9001 with --tls)
```

The gateway of a machine is the part after `@` in its address; map identity -> IP with
`curl -s 'https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true' | jq '.nodes.data[] | select(.ed25519_identity_pubkey=="<identity>") | {ip_addresses, entry}'`.
In the test: tuxi -> `49.12.42.50:9000`, asgard -> `213.218.160.12:9000`.

Analyse a capture (`GW` = that machine's gateway IP):

```sh
P=/data/tmp/chat.pcap; GW=213.218.160.12

# 1. where did this machine send packets? (only gateway, Nym API, DNS resolvers, NTP, LAN broadcast)
tcpdump -nn -q -r $P 'src net 192.168.1.0/24 and not dst net 192.168.1.0/24 and not dst net 224.0.0.0/4' \
  | awk '{print $5}' | sed 's/:$//' | sort | uniq -c | sort -rn | head

# 2. plaintext never appears (use words you actually typed)
grep -a -c "MARKER-TUXI-1" $P                                            # -> 0

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

Wireshark filter for the same view: `ip.addr == 213.218.160.12`; with `--tls` the stream is TLS, without it
`Follow TCP stream` shows the HTTP websocket upgrade followed by binary frames that are already ciphertext.

### Reference run: tuxi <-> asgard, 2026-09-25

Two laptops, two gateways, tcpdump on `wlan0` of each, 184 s session, 4 chat lines (2 each way,
ciphertexts of 81/83/97/90 bytes) plus the wrong-key intruder message:

| | tuxi capture | asgard capture |
|---|---|---|
| packets captured | 124 861 (host also runs other things) | 40 498 |
| packets to / from own gateway | 21 414 / 20 372 | 20 585 / 19 270 |
| hits for the 8 plaintext markers (`MARKER-*`, `secret meeting`, `hello ...`) | 0 | 0 |
| hits for the 5 exact ciphertexts printed by `--show-ciphertext` | 0 | 0 |
| rate to gateway | ~100 packets/s, constant, from connect to disconnect | ~100 packets/s, constant |
| segment sizes to gateway (most to least frequent) | 1428, 1032, 636, 240, 1272 | 1428, 1032, 636, 240, 1272 |
| other remote endpoints (asgard) | | `92.39.63.14:443` Nym API, `1.0.0.1` / `149.112.112.112` / `9.9.9.9` `:443`/`:853` DNS, `:123` NTP |
| readable text in gateway stream | `Host: nymgw1.tinkerbase.org:9000`, `Upgrade: websocket` | `Host: nym-exit.ro-2.silentriver.foo:9000`, `Upgrade: websocket` |

The message rate did not change when lines were typed: the client sends cover packets at the same rate whether
or not there is anything to say. Strings extracted from the gateway stream (`strings -n 8`) are random bytes.

### 4. Trust boundaries

* The peer address (`<peer>.address.secret`) tells the mixnet where to deliver; a wrong or spoofed address only
  causes undecryptable messages on the other side, never plaintext exposure.
* Anyone who obtains `key.secret` can read and forge messages. Keep it 0600, move it only offline, rotate it.
* Deleting `nym-pq-chat-storage/` gives the machine a new Nym identity/address (re-run `init` and re-exchange
  the `.address.secret` file).
