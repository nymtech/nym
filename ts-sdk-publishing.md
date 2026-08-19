# Publishing the TypeScript SDK to npm

This is the npm counterpart to [`crate-publishing.md`](./crate-publishing.md). The two
release flows are independent: nothing here touches crates.io, and the crates.io flow does
not touch npm.

The first half describes how the release machinery is put together. The second half is the
procedure to follow when you actually cut a release.

## Reference

### Prerequisites

The dispatch workflow supplies everything itself. If you run any part of the flow locally
you need Node 24, pnpm, a Rust toolchain with `wasm-pack` and `wasm-opt`, and `jq`, which
both `publish.sh` and `wasm/smolmix/Makefile` shell out to. Publishing locally also needs
an authenticated npm session, either `npm login` or a token in `.npmrc`; the workflow gets
this from the `NODE_AUTH_TOKEN` secret.

### What gets published

Four packages, published from source by `sdk/typescript/scripts/publish.sh`:

| Package | Carries wasm | Depends on |
| --- | --- | --- |
| `@nymproject/mix-tunnel` | yes, base64-inlined | nothing |
| `@nymproject/mix-fetch` | no | `mix-tunnel` |
| `@nymproject/mix-dns` | no | `mix-tunnel` |
| `@nymproject/mix-websocket` | no | `mix-tunnel` |

All four declare `"files": ["dist/**/*"]`, so only each package's own build output is
packed.

`mix-tunnel` has no runtime dependencies, but it does carry `@nymproject/smolmix-wasm` as a
devDependency. pnpm rewrites that to a concrete version at pack time, so the published
manifest names a `smolmix-wasm` version that does not exist on npm. Consumers never install
devDependencies, so this is cosmetic.

### What is never published

`@nymproject/smolmix-wasm` is workspace-internal. Its bytes are base64-inlined into
`mix-tunnel` at build time, and `wasm/smolmix/Makefile` marks the generated
`pkg/package.json` as `private: true` through its `mark-pkg-private` target to enforce
this.

The `sdk/typescript/packages/{sdk,sdk-react,nodejs-client}` directories hold the v1 SDK.
They are absent from the committed `pnpm-workspace.yaml`, but `dev:on` adds the glob
`sdk/typescript/packages/**`, which matches them, so during any build or publish they are
workspace members. Nothing builds or publishes them: `build:ci:sdk` scopes to the four by
name, and they are not in `publish.sh`'s package array. See
`sdk/typescript/packages/sdk/DEVELOPERS.md` for what still builds there.

### The `workspace:*` coupling

`mix-fetch`, `mix-dns` and `mix-websocket` each depend on `mix-tunnel` via `"workspace:*"`.
When `pnpm publish` packs a tarball it rewrites that specifier to the exact sibling version
it built against, so a published `mix-fetch@2.0.0` carries a frozen `mix-tunnel@0.1.0` pin.

All the wasm lives in `mix-tunnel`, and the other three reach it only through that pin.
Bumping `mix-tunnel` alone leaves every dependent pinned to the old wasm. Whenever
`mix-tunnel` is republished, the three dependents must be republished too so their pins
advance.

This is also why the flow uses `pnpm publish` rather than `npm publish`. npm leaves the
literal string `workspace:*` in the tarball, producing packages that fail to install.
`publish.sh` turns dev mode on so the siblings are in the workspace during packing, and a
trap restores `pnpm-workspace.yaml` on exit.

### CI workflows

| Workflow | Trigger | Builds the four packages | Role |
| --- | --- | --- | --- |
| `ci-lint-typescript` | PR touching `sdk/typescript/**`, `ts-packages/**`, wallet, explorer | yes, via `pnpm build:ci` | the gate that actually compiles them |
| `ci-build-ts` | PR touching `sdk/typescript/**`, `ts-packages/**` | no | `pnpm build` plus storybook, uploaded to S3 |
| `ci-sdk-wasm` | PR touching `wasm/**`, `common/**`, `clients/client-core/**` | wasm only | `sdk-wasm-build`, `-test`, `-lint` |
| `ci-docs-typedoc-fresh` | push touching the four packages' `src/**` or the committed `api/**` | builds them, to resolve re-exports | fails if the committed API reference is stale |
| `ci-sdk-example-integration-tests` | schedule and dispatch | no | Rust SDK examples |
| `publish-sdk-npm` smoke step | part of the dispatch below | uses the built release wasm | the only check that runs the wasm in a browser |
| `publish-sdk-npm` | dispatch only | yes, via `pnpm sdk:build` | builds and publishes |

`ci-docs-typedoc-fresh` is a gate rather than a generator. It regenerates the API
reference with `pnpm docs:typedoc` and fails if the result differs from what is committed,
so any change to the four packages' `src/**` has to be accompanied by a regenerated
`documentation/docs/pages/developers/*/api` tree in the same push. It builds the wasm and
the four packages before running typedoc, so cross-package re-exports resolve against fresh
`dist`, and `generate-typedoc.sh` pins `--gitRevision develop` to keep source links stable.

`make sdk-wasm-test`, which `ci-sdk-wasm` runs as its "Test" step, has an entirely
commented-out body and does nothing. `sdk-wasm-lint` does compile for
`wasm32-unknown-unknown`, so it catches type and lint errors, but a host-only API such as
`std::time::Instant::now()` compiles fine there and panics only when called. Nothing
except the smoke test catches that class of bug.

The naming is misleading. `ci-build-ts` runs `pnpm build`, which resolves to
`build:types build:packages` and covers only `@nymproject/types`, `mui-theme` and `react`.
The workflow that compiles the four published packages before merge is
`ci-lint-typescript`, because it runs `pnpm build:ci`, which includes `build:ci:sdk`. If
you are checking whether a change to the published packages is covered by CI, look there.

### The publish workflow

`.github/workflows/publish-sdk-npm.yml`, `workflow_dispatch` only. Two inputs:

| Input | Default | Effect |
| --- | --- | --- |
| `dry_run` | `true` | runs `pnpm publish --dry-run`; nothing is uploaded |
| `dist_tag` | `auto` | `auto` derives a tag per package; `next` or `latest` forces one on all four |
| `skip_smoke` | `false` | publishes even if the browser smoke test fails |

Before publishing, the workflow brings the tunnel up in chromium and firefox using the
release wasm that `pnpm sdk:build` just produced. It is the only check that executes the
wasm rather than compiling it, and it runs on dry runs too.

It talks to a real gateway and IPR, so it can fail for reasons unrelated to the release.
`skip_smoke` is for that case only.

Under `auto`, `publish.sh` resolves each package's tag from what is already on npm:

| Situation | Tag |
| --- | --- |
| version carries a prerelease suffix | `next` |
| package is not yet on npm (`npm view` returns 404) | `latest` |
| major matches the current `latest` major | `latest` |
| major is higher than the current `latest` major | `next` |
| major is lower than the current `latest` major | `next` |

A higher major goes to `next` so existing users stay on `latest` until you promote it with
`npm dist-tag add <pkg>@<version> latest`. Once `latest` points at the new major, ordinary
patches resolve to `latest` again on their own.

If `npm view` fails for any reason other than a 404, `publish.sh` aborts rather than
guessing. Defaulting to `latest` on a network error could push a breaking major onto every
current consumer.

The workflow runs `pnpm sdk:build`, which rebuilds the wasm from local source
(`build-prod-sdk.sh` calls `pnpm build:wasm`, which calls `make sdk-wasm-build`). The
published bytes reflect the current branch, independent of what is on crates.io.

`make sdk-wasm-build` also builds `wasm/client`, which none of the four packages need. The
`files` allowlist keeps it out of every tarball, so it costs a few minutes of build time
and nothing else.

`build-prod-sdk.sh` has no trap around its own `dev:on` and `dev:off` pair, and it runs
`pnpm install --no-frozen-lockfile` between them. So `pnpm sdk:build` modifies
`pnpm-lock.yaml` in your working tree, and if it dies mid-build it leaves
`pnpm-workspace.yaml` dirty. Run `pnpm dev:off` and check `git status` after a failed
build. Only `publish.sh` restores the workspace unconditionally.

`sdk/typescript/scripts/release.sh` is the local equivalent. It calls `make
sdk-wasm-build`, then `pnpm sdk:build` (which runs `pnpm build:wasm` again, building the
wasm twice), then `publish.sh`. It never sets `DRY_RUN`, which `publish.sh` defaults to
`0`, so it publishes for real with no rehearsal. Prefer the workflow.

### Version-bump tooling

`tools/internal/sdk-version-bump` edits many `package.json` files and the two wasm crates'
`Cargo.toml` in one pass. It is wired into the root `package.json`:

| Script | Command | Intended effect |
| --- | --- | --- |
| `pnpm sdk:versions:bump` | `cargo run -p sdk-version-bump bump-version` | `X.Y.Z` to `X.Y.(Z+1)`, final, no `-rc` |
| `pnpm sdk:versions:add-rc` | `cargo run -p sdk-version-bump --pre-release` | bump an existing `-rc.W` prerelease |
| `pnpm sdk:versions:remove-rc` | `cargo run -p sdk-version-bump remove-suffix` | strip `-rc.N` where present |

`bump-version` moves versions straight to the next final patch. It does not put `-rc.0` on
the version field, despite what its `--help` text suggests. The `-rc.0` appears only in
`@nymproject/*` dependency floor specifiers, which is this repo's normal style.
`workspace:*`, `file:`, `link:` and `npm:` specifiers are skipped by design, since pnpm
resolves them at pack time. A `--ts-only` flag exists for the case this flow hits most
often, a TypeScript release with no underlying wasm change: it leaves the two wasm crates'
`Cargo.toml` versions alone.

The tool cannot currently be run, and has several known problems:

1. It is commented out of the workspace. `Cargo.toml` has
   `# "tools/internal/sdk-version-bump"` in `[workspace.members]`, so
   `cargo run -p sdk-version-bump` fails with "not found in workspace". Re-enabling it as a
   plain member pulls `git2`, `openssl-sys`, `libgit2-sys`, `native-tls` and their
   dependencies into the workspace `Cargo.lock` through `cargo-edit`'s library, none of
   which are there today. It also pins `cargo-edit = "0.11.0"` while the root workspace
   declares `0.13.8`, so a plain member puts two versions of `cargo-edit` in the graph.
   Making it a standalone excluded crate with its own lockfile, run via
   `cargo run --manifest-path`, would leave the workspace build alone.
2. It double-bumps `mix-fetch`. The registered path list holds both `packages/mix-fetch`
   and `packages/mix-fetch/internal-dev`; the latter has no `package.json` of its own, so
   the finder walks up to `mix-fetch/package.json` and bumps it a second time, taking
   `2.0.0` to `2.0.2`.
3. It reformats every `package.json`. The JSON path round-trips through `serde_json`, which
   reorders keys and expands inline arrays, producing large diffs unrelated to the version
   change. The Rust path uses `toml_edit` and stays clean.
4. It bumps far more than the four published packages, including `nym-wallet`, the v1 SDK
   directories and several examples. None of those are published by `publish.sh`.
5. `sdk:versions:add-rc` is malformed. It lacks both the `--` separator that hands
   remaining arguments to the binary and the `bump-version` subcommand that the other two
   scripts supply. The intended invocation is
   `cargo run -p sdk-version-bump -- bump-version --pre-release`. Problem 1 masks this,
   since the tool cannot run at all.

Until these are fixed, edit the four version fields by hand. It is four one-line changes
and produces a clean version-only diff.

### Differences from the crates.io flow

| | crates.io | npm |
| --- | --- | --- |
| Versioning | lockstep on one `[workspace.package]` version | independent per package |
| Version bump | `ci-crates-version-bump`, which opens a PR | hand-edited |
| Dry run | a separate `ci-crates-publish-dry-run` workflow | the `dry_run` input, on by default |
| Cross-package deps | `workspace = true` resolved at publish | `workspace:*` rewritten by pnpm at pack time |
| Docs constants | the bump workflow seds `versions.ts` | updated by hand |

## How to publish

### 1. Bump the four versions

Edit the `version` field in each of:

```
sdk/typescript/packages/mix-tunnel/package.json
sdk/typescript/packages/mix-fetch/package.json
sdk/typescript/packages/mix-dns/package.json
sdk/typescript/packages/mix-websocket/package.json
```

Leave the `"@nymproject/mix-tunnel": "workspace:*"` dependency lines alone. pnpm rewrites
them at pack time.

Bump all four even if only `mix-tunnel` changed, so the dependents' pins advance to the new
wasm.

Check what is actually on npm before choosing the new numbers, rather than incrementing
what is in the tree:

```bash
for p in mix-tunnel mix-fetch mix-dns mix-websocket; do
  printf '%-14s ' "$p"; npm view "@nymproject/$p" dist-tags --json
done
```

The versions committed to `develop` can lag the registry, because a release published from
a branch leaves nothing behind if step 3 is skipped. If the tree says `0.1.0` and npm says
`0.1.1`, bumping the tree gives you `0.1.1`, which is already taken, and `publish.sh` aborts
on the 403.

### 2. Rehearse from your branch

Push the branch, then dispatch the workflow against it with `dry_run` left on:

```bash
git push -u origin <your-branch>
gh workflow run publish-sdk-npm.yml --ref <your-branch> -f dry_run=true -f dist_tag=auto
```

The ref must already exist on origin or the dispatch fails with `HTTP 422: No ref found`.
This runs the full wasm and TypeScript build and a dry-run publish against your bumped
versions, without merging or uploading anything.

Check the "Summary of packages to publish" block in the log and confirm each resolved tag
is the one you expect from the resolution table above, rather than assuming the four should
match each other. Work it out per package from the versions you saw in step 1: same major
as the published `latest` gives `latest`, a prerelease or a major that has crossed the
published one gives `next`.

If a tag is not what you expected, fix the version. Do not force `dist_tag = latest` to
make the four agree: on a package whose major has crossed, that moves `latest` onto the new
major and breaks every existing consumer of the old one.

### 3. Merge the bump

Merge into the branch you will publish from, normally `develop`. The workflow checks out
the branch it is dispatched against, so the bumped versions have to be committed there
first.

### 4. Publish

Dispatch `publish-sdk-npm` from `develop` with `dry_run` unticked and `dist_tag` on `auto`.

`publish.sh` walks the four packages in a fixed order under `set -o errexit`, starting with
`mix-tunnel`. It is not idempotent: a package whose version is already on npm returns 403
and aborts the whole run. There is no resume workflow, unlike the crates.io flow's
`ci-crates-publish-resume`.

The failure that matters is a partial run. `mix-tunnel` goes first, so an abort part way
through leaves it published and some dependents not, which is the stranded-pin state
described above. If a run aborts, check what actually landed on npm and publish the
remaining packages by hand from their directories with
`pnpm publish --access=public --no-git-checks --tag <tag>`.

### 5. Promote, if you published a new major under `next`

```bash
npm dist-tag add <pkg>@<version> latest
```

Promoting does not clear the `next` tag, and nothing else does either. A `next` left behind
from an earlier major can end up pointing at an older release than `latest`, so
`npm i <pkg>@next` installs something staler than a bare `npm i`. Re-run the `npm view`
loop from step 1; if a package's `next` now lags its `latest`, either move it forward or
drop it. Note the tag name is the last argument, so advancing `next` is not the command
above:

```bash
npm dist-tag add <pkg>@<version> next   # move next forward
npm dist-tag rm <pkg> next              # or remove it
```

### 6. Update the docs constants

Edit `documentation/docs/components/versions.ts` to match what you published:
`MIX_FETCH_VERSION`, `MIX_TUNNEL_VERSION`, `MIX_DNS_VERSION`, `MIX_WEBSOCKET_VERSION`.

Nothing does this automatically. `ci-crates-version-bump` seds the Rust constants in the
same file but deliberately leaves the TypeScript ones alone. Note that these four are
currently imported by nothing, so the step is bookkeeping to keep the file honest for
whenever a docs page does start reading them.
