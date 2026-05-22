# PR #6784 — queued replies (no code change needed)

Tracker for review-comment responses we've identified where the right outcome is a written reply rather than a code edit. Each entry has:

- **Author + comment id** — for direct linking
- **Anchor** — the file:line the comment is on
- **Original gist** — one-line summary of the reviewer's point
- **Reply draft** — text ready to paste onto the GitHub thread
- **Status** — `draft` / `sent`

---

## 1. CR #3247977247 — workspace consumers `default-features = true`

- **Anchor:** root `Cargo.toml` (workspace declarations)
- **Original gist:** "All workspace consumers must explicitly set `default-features = true` for `nym-ip-packet-requests` and `nym-lp` — workspace decl is `false`."
- **Status:** draft

**Reply:**

> Thanks — the premise is inverted here. The current workspace declarations are:
>
> ```toml
> nym-ip-packet-requests = { version = "1.21.0", path = "common/ip-packet-requests" }
> nym-lp = { version = "1.21.0", path = "common/nym-lp" }
> ```
>
> Neither sets `default-features = false`. Consumers using `{ workspace = true }` inherit defaults-on. Audited every consumer:
>
> - `sdk/rust/nym-sdk`, `nym-gateway-probe`, `nym-ip-packet-client`, `smolmix/core`, `common/nym-connection-monitor`, `common/registration`, `service-providers/ip-packet-router`, `nym-node` — all either explicitly set `default-features = true` or inherit defaults and are fine with them.
> - `tools/nym-lp-client`, `gateway`, `nym-registration-client`, `integration-tests` — path-based, inherit the crate's own `default = ["full"]`.
> - `wasm/smolmix` — was the one consumer disabling `nym-lp` defaults (libcrux-psq / nym-kkt don't compile on wasm32). Post-rebase it migrated to `nym-lp-data` directly (upstream #6810), which sidesteps the question entirely.
>
> If the policy goal is "workspace decls should explicitly set `default-features = false` so each consumer must opt in," that's a defensible architectural choice and we can do it as a follow-up — but it's a wider change than this PR, and not the current state.

---

## 2. CR #3247977245 — `rustls-rustcrypto = "0.0.2-alpha"` justification

- **Anchor:** root `Cargo.toml:345`
- **Original gist:** "Replace with a production-ready cryptography provider."
- **Status:** draft

**Reply:**

> The smolmix-side Cargo.toml already documents the rationale (`wasm/smolmix/Cargo.toml:27-28`):
>
> ```toml
> # TLS: rustls-rustcrypto is the only viable provider on wasm32-unknown-unknown.
> # ring + aws-lc-rs both fail to compile (no OS entropy / no C toolchain in browser).
> ```
>
> Happy to add a forward-compat pointer at the workspace-level declaration too if reviewers landing here first would appreciate it — see follow-up comment if applied.

---

## 3. CR #3247977262 — `dns.rs` first-datagram bug

- **Anchor:** `wasm/smolmix/src/dns.rs` (UDP recv loop)
- **Original gist:** "Don't treat the first UDP datagram as the active query's answer — drop and keep reading on ID mismatch."
- **Status:** draft

**Reply:**

> Fix is in place — see `dns.rs` `query_record`. The recv loop now has three drop-and-continue gates, all logging instead of failing:
>
> 1. Source address must match the `server` we queried (anti-spoof layer 1, also catches late replies from earlier CNAME hops / fallback retries on the reused socket).
> 2. `Message::from_vec` parse failures (anti-spoof layer 2 — a malformed packet shouldn't abort a live query).
> 3. Transaction ID match against the CSPRNG-generated `query_id` (anti-spoof layer 3).
>
> Timeout still fires `FetchError::Timeout` if no matching response arrives within `DNS_TIMEOUT`.

---

## 4. JS #3274509964 — IPR `parse_incoming` Err handling

- **Anchor:** `wasm/smolmix/src/bridge.rs:145`
- **Original gist:** "how do we ensure the message we got is actually from the IPR we think we're talking with?"
- **Status:** draft

**Reply:**

> Genuinely an open design question — I've flagged it in our internal action list as the IPR-authentication thread to discuss. The CR comment above suggested terminating the tunnel on every `parse_incoming` Err; my objection was that this hands an attacker a free DoS (one spoofed packet → session aborted). Your wider framing ("how do we know the message came from the IPR we expected?") makes the case for an authenticated handshake rather than relying on packet shape.
>
> Two follow-up options worth a design call:
>
> 1. Per-session HMAC bound to the IPR's recipient, verified before the parse path runs.
> 2. Treat the `Recipient` field on `ReconstructedMessage` as ground truth (if it's reliably available post-mixnet hop), and filter at the bridge boundary.
>
> Neither belongs in this PR. Linking this thread to the related `ipr.rs:90` reply.

---

## 5. JS #3274896702 — `ipr.rs:90` bail on decode failure

- **Anchor:** `wasm/smolmix/src/ipr.rs:90`
- **Original gist:** "if we fail to decode it, shouldn't we bail because we got garbage?"
- **Status:** draft

**Reply:**

> The literal patch (`continue` → `return Err(...)`) would introduce a DoS vector: the `reconstructed_receiver` carries all mixnet traffic during the handshake (cover traffic, unrelated replies, late hops from other sessions). A single mixed-in non-LP packet would abort the handshake. Filtering by continuing on decode failure is the correct shape; the deeper question is the IPR-authentication design call linked above (CR #3247977261 / your #3274509964).
>
> If you want a smaller mid-ground: add a debug log on the `continue` so we don't silently swallow malformed input during diagnostics. Happy to do that as a follow-up.

---

## 6. JS #3274977301 — `mixsocket.rs:66` Sec-WebSocket-Protocol header gating

- **Anchor:** `wasm/smolmix/src/mixsocket.rs:66`
- **Original gist:** "shouldn't that header only be included if user explicitly asked for wss rather than ws?"
- **Status:** draft

**Reply:**

> `Sec-WebSocket-Protocol` is a WebSocket-handshake-level header (RFC 6455 §4.1) for subprotocol negotiation. It's transport-independent — it applies identically to `ws://` and `wss://` because the negotiation happens after the HTTP upgrade, regardless of whether TLS wraps the underlying TCP stream.
>
> The current code only sets the header when the caller has actually provided subprotocols (`if !protocol_list.is_empty()`), so the behaviour is already conditional on user intent rather than on scheme. Happy to add a comment explaining this if the next reader would benefit.

---

## 7. JS #3274677136 — `util.rs:23` overlap with `wasm-common-utils`

- **Anchor:** `wasm/smolmix/src/util.rs:23`
- **Original gist:** "didn't we already have those helpers in wasm-common utils?"
- **Status:** draft

**Reply:**

> Checked `common/wasm/utils/src/lib.rs` — no overlap. The pieces in smolmix's `util.rs`:
>
> - `debug_log!` / `debug_error!` — feature-gated wrappers around the upstream `nym_wasm_utils::console_log!` / `console_error!`. They add the `debug` cargo feature gate so verbose logging compiles out by default. Upstream has the unconditional `console_log!` / `console_error!` macros but no feature-gated variants.
> - `hex_preview` — buffer-to-hex helper for binary debug logs. No equivalent upstream.
>
> So `util.rs` extends rather than duplicates. Happy to upstream the feature-gated variants + `hex_preview` into `nym-wasm-utils` as a separate PR if that's preferred for reuse across other wasm crates.

---

## 8. JS #3274924010 — `dns.rs:154` "what's the point?"

- **Anchor:** `wasm/smolmix/src/dns.rs:154` (at JS's review commit) — the line was `let id: u16 = rand::random();`
- **Original gist:** Asking why we override hickory-proto's auto-generated DNS transaction ID with a CSPRNG value.
- **Status:** draft

**Reply:**

> The override is deliberate hardening. `hickory-proto`'s auto-id is not documented to be CSPRNG-quality; depending on version it may be PRNG-based or monotonic. Overriding with `rand::random()` routes through `getrandom` → `crypto.getRandomValues()` on wasm32, which gives us a CSPRNG-backed 16-bit id.
>
> Why it matters: with socket reuse across CNAME hops and concurrent lookups serialised on the `dns_lock`, an attacker who can inject onto the loopback would otherwise have a higher base-rate of guessing an active query's id. The CSPRNG fill reduces that to the theoretical minimum (1/65536 per attempt).

---

## 9. JS #3274722354 — `Cargo.toml:60` "can we not get rid of the 0.2 dep?"

- **Anchor:** `wasm/smolmix/Cargo.toml:60` — the `# crypto.getRandomValues() backend for the 0.2 line.` comment
- **Original gist:** "I think I know the answer to this question, but can we not get rid of the 0.2 dep?"
- **Status:** draft

**Reply:**

> Short answer: no, not without an upstream change. The chain is:
>
> ```
> getrandom 0.2.16
> └── ring 0.17.14
>     └── rustls 0.23.37
>         └── sqlx-core 0.8.6 [build-dependencies]
>             └── nym-client-core-gateways-storage
>                 └── nym-client-core
>                     └── nym-wasm-client-core
>                         └── smolmix-wasm
> ```
>
> `ring` is pinned against `getrandom` 0.2; `rustls 0.23.x` uses `ring` for its `sqlx` integration; `sqlx` is a build-dep of `nym-client-core-gateways-storage`. Smolmix can't drop the 0.2 dep without one of:
>
> 1. Removing `sqlx` from `nym-client-core-gateways-storage`,
> 2. Switching `sqlx` to a non-ring TLS backend (`aws-lc-rs` also requires C toolchain → not wasm-clean; `rustls-rustcrypto` doesn't have sqlx integration),
> 3. Upgrading `ring` (the project is in caretaker mode upstream).
>
> The `[target.wasm32.dependencies.getrandom]` stanza with `features = ["js"]` in smolmix's Cargo.toml is just configuring the right backend for a dep cargo has already decided to include via the build graph. The 0.4 line (`getrandom04 = { features = ["wasm_js"] }`) coexists because `rand_core` 0.10 uses the renamed feature flag.
>
> Worth a workspace-wide cleanup pass at some point, but not in scope for #6784.

---

## How to use this list

When you're ready to do the reply pass:

1. Open the PR conversation tab in the browser.
2. For each entry above (in order), find the linked comment, paste the **Reply** block, hit submit.
3. Flip the entry's **Status** to `sent` (or delete the entry if you're done with it).
4. If a reply gets a meaningful counter-response, copy that thread to a follow-up entry here so we don't lose it.

If a code-level decision falls out of any reply (e.g. you decide to add the source-address check for #3 or the debug log for #5), spin it off into `action-list.md` rather than tracking it here — this file is replies only.
