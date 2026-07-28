# Design: nym-prefix-smol-crates

## D1. Product name vs crate name

Not every crate gets the `nym-` prefix in prose. The rule is whether the crate
has a **separate product identity**:

| Crate | Product identity | Prose name | Crate name |
|---|---|---|---|
| `smol-core` | none (library) | `nym-smol-core` | `nym-smol-core` |
| `smolmix` | yes (`/developers/smolmix` docs page + `@nymproject/mix-*` TS SDKs) | smolmix | `nym-smolmix` |
| `smoldvpn` | none (library) | `nym-smoldvpn` | `nym-smoldvpn` |

So `smolmix` stays "smolmix" in docs prose, the docs page URL, and the nav; only
the installable crate becomes `nym-smolmix` (the install snippet, `docs.rs`
link, and `cargo run -p`). This mirrors the TS side, where the product is
"smolmix" but the npm packages are `@nymproject/mix-*`.

## D2. Directory names unchanged

The repo is already mixed: `common/crypto` → `nym-crypto`, but `common/nym-kcp`
→ `nym-kcp`. So a directory move is not required; renaming only the `[package]
name` avoids a `git mv` and keeps the diff to manifests + references.

## D3. `nym-smolmix-wasm` pins its artifact (Option B)

The wasm crate is never published to crates.io; its bytes are base64-inlined into
`@nymproject/mix-tunnel`. Two options existed:

- **A (full derivation):** let the artifact follow the crate →
  `nym_smolmix_wasm_bg.wasm` + `@nymproject/nym-smolmix-wasm`, updating the
  mix-tunnel imports, webpack, and a silent-failure string-replace in
  `rollup/worker.mjs`, then re-verifying in a browser.
- **B (pin, chosen):** rename only the crate; keep the artifact basename
  (`--out-name smolmix_wasm`) and the private npm name (`mark-pkg-private` stamps
  `name: "@nymproject/smolmix-wasm"`). The TS/build wiring is untouched.

B is consistent with D1 (crate prefixed, artifact keeps its established name) and
avoids the `worker.mjs` trap. Its one dependency is that `mark-pkg-private` runs;
made robust by ordering it after `build-rust` in the recipe (so `make -j` cannot
race the wasm-pack write) and failing loudly (pnpm can't resolve
`@nymproject/smolmix-wasm` if the pin is missing).

## D4. Live specs: Purpose direct, requirement bodies via delta

Per the OpenSpec convention (and the prior `smoldvpn-rename` change), the live
spec **Purpose** line is outside the delta grammar and is edited directly, while
**requirement bodies** are updated only through a change delta and applied at
archive time. So the live specs carry `nym-*` on their Purpose lines now, and
this change's deltas carry the renamed requirement bodies; `openspec apply`
propagates them.

## D5. What does NOT change

Type names (`SmolCoreError`, `SmolmixError`), the `smol-core-stack` capability
name, the `smoldvpn-*` example binary identifiers, the published `@nymproject/mix-*`
package names/APIs, and all gateway/protocol behaviour. This is a naming change
only.
