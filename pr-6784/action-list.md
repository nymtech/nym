# PR #6784 — action list (2026-05-21)

Sources: CodeRabbit (17 top-level inline + 8 buried-in-review-summary) + jstuczyn (39 inline + review-body summary) + your close_notify ask.

Notation: **CR** = CodeRabbit, **JS** = jstuczyn, ID = github comment id.

---

## A. Quick-win nits (≤ 5 min each — batch them)

| File:line | Source | What |
|---|---|---|
| `wasm/smolmix/Makefile:1` | CR #3249205863 | Declare command targets as `.PHONY`. |
| `wasm/smolmix/Makefile` | CR #3247977258 | Drop hard dependency on `taskset`. |
| `wasm/smolmix/README.md:53` | CR #3249205868 | Add language identifiers to fenced code blocks (markdownlint MD040). |
| `wasm/smolmix/src/mixdns.rs:1` | JS #3274665205 | Copyright header is copy-pasted — global s&r across new files. |
| `wasm/smolmix/Cargo.toml:5` | JS #3274700226 | Inherit `edition`, `rust-version` etc. from workspace. |
| `wasm/smolmix/src/bridge.rs:95` | JS #3274857432 | Log packet count alongside the existing log. |
| `wasm/smolmix/src/bridge.rs:142` | JS #3274877457 | Same — log count. |
| `wasm/smolmix/src/stream.rs:237` | JS #3274958041 | Should be a `static`, not a `const`. |
| `wasm/smolmix/src/stream.rs:293` | JS #3274964476 | Write `65535` instead of `u16::MAX - …`. |
| `wasm/smolmix/src/http.rs:219` | JS #3275000888 | Remove the noisy log/branch. |
| `wasm/smolmix/src/fetch.rs:131` | JS #3275010139 | Early-return on non-2xx-3xx, collapse the if. |
| `wasm/smolmix/internal-dev/headless.js:26` | CR (review-body Minor) | Clamp `count` URL param to `[1, 500]`. |
| `wasm/smolmix/.cargo/config.toml` (or similar) | CR (review-body Minor) | Remove unsupported `[build].target_arch` key. |

---

## B. Cargo / dependency correctness

| File | Source | What |
|---|---|---|
| `Cargo.toml` workspace consumers | CR #3247977247 (🔴 Critical) | Every consumer of `nym-ip-packet-requests` and `nym-lp` must explicitly set `default-features = true` now that the workspace decl is `false`. Hit list: `wasm/smolmix`, `tools/nym-lp-client`, `nym-registration-client`, `nym-gateway-probe`, `nym-ip-packet-client`, `integration-tests`, `gateway`. |
| `Cargo.toml:345` | CR #3247977245 | Add an in-`Cargo.toml` comment explaining why `rustls-rustcrypto = "0.0.2-alpha"` is intentional (wasm32 has no other viable pure-Rust provider — see [[wasm-rustls]]). CR's softer reply accepts the choice; the ask is just a forward-compat comment for future readers. |

---

## C. Reactor cleanup

| File:line | Source | What |
|---|---|---|
| `reactor.rs:45` | CR #3247977301 | Use a monotonic clock for smoltcp time. |
| `reactor.rs:103` | CR #3247977306 + JS #3274563141 (both agreed) | When the notify channel closes, **set the shutdown flag** and break the loop so the bridge stops too. |
| `reactor.rs:64` / `reactor.rs:49` | JS #3274573204 + #3274799363 | Replace `UnboundedReceiver<()>` with `Arc<Notify>` — coalescing is fine, 10 notifications behave identically to 1. |

---

## D. Bridge correctness

| File:line | Source | What |
|---|---|---|
| `bridge.rs:67` | JS #3274849462 | Race: `select!` between message+tick can starve the shutdown branch. Add the shutdown receiver into the same `select!` so we exit promptly. |
| `bridge.rs:145` (IPR `parse_incoming` Err) | CR #3247977261 + JS #3274509964 + your reply | **Moved to open questions** — see §H.1. |

---

## E. Stream lifecycle

| File:line | Source | What |
|---|---|---|
| `stream.rs:132` / `stream.rs:117` | CR #3247977312 + JS #3274600343 / #3274954241 (all agreed) | Add `closed: bool` state to `WasmTcpStream`. On Drop, call `socket.close()` and return `Poll::Pending` until smoltcp actually transitions to `Closed` — don't `abort()`. |
| `stream.rs:353` | CR #3247977317 | When a TCP connect fails, remove the socket from `SocketSet` instead of leaving it dangling. |

---

## F. Tunnel hardening

| File:line | Source | What |
|---|---|---|
| `tunnel.rs:289` | CR #3247977332 + JS #3274607398 (agreed: "simple enough, do it") | Add IPv6 address + default route to the smoltcp interface (matching the existing IPv4 block). |
| `tunnel.rs` | CR #3247977322 | `shutdown()` must actually release the base client (currently it doesn't). |
| `tunnel.rs:200` | CR #3247977329 | Don't swallow `gateway-storage` errors into "no gateway" — propagate so callers can distinguish missing-config from storage-broken. |
| `lib.rs:117` | CR #3247977294 + JS #3274534208 | Pick one: error on duplicate `setup()` *or* silently no-op. CR's note: `OnceLock::get_or_init` is sync-only so can't wrap `WasmTunnel::new` directly. Suggest **error** — surfaces caller bugs. |

---

## G. Fetch / HTTP / DNS / IPR

| File:line | Source | What |
|---|---|---|
| `fetch.rs:330` | CR #3247977276 | Multi-value response headers collapse to last entry — switch to `HashMap<String, Vec<String>>` or use `headers.append` semantics. |
| `fetch.rs` (retry path) | CR #3247977269 (🏗️ Heavy) | Retry-on-stale could replay non-idempotent requests. Restrict the retry to "connection error before bytes are written" — needs a flag on the conn type. |
| `dns.rs` | CR #3247977262 + you replied "made changes, review" | Verify the fix landed and CR's concern is gone: "first UDP datagram is not necessarily the answer to the active query." |
| `dns.rs:20` | JS #3274909742 + you agreed | Make resolver IP configurable; pick a default that isn't 8.8.8.8 (audience-sensitive). |
| `ipr.rs:90` | JS #3274896702 | If decode fails, bail — currently silently continues on garbage. |
| `mixsocket.rs:66` | JS #3274977301 | Only include the `Sec-WebSocket-…` upgrade header when scheme is `wss`, not `ws`. |
| `util.rs:23` | JS #3274677136 | Check if helpers already exist in `wasm-common`/`wasm-utils` — import instead of redefining. |
| `device.rs:31` | JS #3274904468 + you replied "double-checking notes" | MTU: set to sphinx payload size to avoid fragmentation. Verify reason first. |

---

## H. Close-notify Hyper fix (your add)

Per MEMORY (2026-05-14): TLS `close_notify` on `read_until_eof` appeared resolved post-hyper-1.x rewrite, with 201 Created end-to-end on jsonplaceholder. But it's been intermittent — listed as a PITA. Two angles to action today:

1. **Reproduce** with a deterministic POST against jsonplaceholder + a known-flaky CDN (e.g. one that closes without `close_notify`). Get a clear pass/fail.
2. **Harden the read loop** in `http.rs`: catch the specific `rustls` `UnexpectedEof` and treat it as a clean EOF if the body length was satisfied (Content-Length or chunked terminator already seen). Otherwise propagate. This is the standard "tolerate non-conforming TLS close" pattern that hyper itself applies for HTTP/1.1.

---

## I. Internal-dev / testing surface (CR review-body minors)

| File | What |
|---|---|
| `internal-dev/*.js` (or similar JS test harness) | Check for existing WebSocket connection before creating a new one. |
| Same | Memory leak: console listener not removed on timeout. |
| Same | Race: early `ws-send` messages silently dropped before WS opens — queue or await `onopen`. |
| Same | Guard against uninitialised WASM (probably `if (!wasm) throw …` at JS entry points). |
| `common/ip-packet-requests/src/codec.rs:175` | Add `#[cfg(feature = "codec")]` to the `#[cfg(test)]` module so `--no-default-features` builds compile. |

---

## OPEN QUESTIONS (flag for discussion — don't action today)

1. **IPR message authentication.** CR #3247977261 + JS #3274509964 + your reply form one thread. CR's narrow point (terminate on `parse_incoming` Err) collides with your DoS objection (one spoofed packet could kill a session) and JS's wider question ("how do we know the message came from the IPR we think we're talking with?"). Resolution = design call: do we add a per-session HMAC / recipient-identity check, or do we live with the existing trust model and just classify Errs differently? **Linked to JS's review-body note: "revisit all loops where we get data from IPR — continue on errors or just bail."**
2. **Per-feature gating in `smolmix` Cargo.toml.** JS #3274717097: split into `dns` / `ws` / `fetch` features so TS SDK packages only pull what they use. You agreed in principle. Big refactor — defer unless we want it in this PR.
3. **Atomic outbound sequence number.** JS #3274864339: does it have to be `AtomicU16`/`AtomicU64`? Single-writer = no. Cheap to fix once we confirm only one task writes.
4. **Random starting sequence.** JS #3274873152: TCP randomises ISN; should our outbound seq do the same? Threat-modelling question.
5. **SmoltcpStack inner-pattern refactor.** JS #3274792681: wrap `iface/sockets/device` in `Arc<Mutex<Inner>>` to make passing around safer. Cleaner but invasive — out of scope for today probably.
6. **Replace 1ms sleep with the `yield` primitive** you found earlier. JS #3274823266 — what was it? `wasm-bindgen-futures::yield_now()`? Worth a one-line answer + commit.
7. **Smarter SURB policy.** JS #3274890748 + your reply: is 2 SURBs/request the right shape, or should the client's background bucket handler cover it? Needs alignment with @simonwicky.
8. **UDP-from-browser direction.** JS #3274945963: smolmix UDP could let browsers do "normal" UDP traffic. Feature-scope decision.
9. **Builder for configurable timeouts.** JS #3274967301 + you agreed: nice-to-have, scope.
10. **Configurable storage encryption.** JS #3274760859: allow clients to opt-in to encrypted storage (requires password). Feature-scope decision.
11. **Clear `OnceLock` on shutdown.** JS #3274767952: should `setup()` be re-callable after shutdown, or stay one-shot-per-page-load? Either is fine; pick one.
12. **Drop the `0.2` dep.** JS #3274722354 references `Cargo.toml:60` — need to look at which dep specifically. Probably `pin-project-lite`? Confirm what was pinned.
13. **Refactor `nym-lp` packet to its own crate.** JS #3274648320 + cc'd @simonwicky. Cross-team decision.
14. **Tunnel task health tracker.** JS review-body §3: "some equivalent of a task tracker that makes sure all tasks required by the tunnel are still healthy, because if one fails the rest become useless." Likely the right answer is a `JoinSet` (or wasm equivalent) with shutdown-on-first-error semantics. Design call.
15. **`dns.rs:154`.** JS #3274924010: "what's the point?" — needs reading the actual line in context and writing back. Possibly a stale code path.
16. **`fetch.rs:31` — broader fetch-spec adherence (CORS etc).** JS #3275013921: scope decision.
17. **`http.rs:96` — "use a library for formatting".** JS #3274992974: which crate fits (httparse, http-body-util, hyper itself already pulled in)?

---

## SUGGESTED ORDER FOR TODAY

1. §A nits (one commit, fast warm-up).
2. §B Cargo correctness (avoid breaking downstream).
3. §H close_notify reproduction first, then hardening — if repro is flaky, descope to "added defensive UnexpectedEof handling" with a manual test note.
4. §C reactor cleanup (Notify + monotonic clock + shutdown-flag-on-close).
5. §D bridge shutdown-race fix.
6. §E stream graceful-close state machine.
7. §F tunnel IPv6 + shutdown release + gateway-storage err.
8. §G fetch/dns/ipr — pick off the ones you replied "agreed" or "made changes, review" first.
9. §I internal-dev minors if time.

Stop at §H if anything in 4-7 surfaces unexpected. Don't open the OPEN QUESTIONS list before we talk through it together.
