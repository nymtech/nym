/**
 * Centralised version constants for documentation.
 *
 * Components (VersionBanner, CodeVerified, etc.) import from here so there is a
 * single place to update on each release.
 *
 * Fenced code blocks still hardcode versions for copy-paste friendliness.
 * The Cargo.toml snippets are in .nym-codex/snippets/developers/. The crate
 * release workflow (ci-crates-version-bump.yml) rewrites their versions with
 * the constants here, and documentation/scripts/verify-doc-versions.sh fails
 * the docs CI when a snippet and a constant differ.
 *
 * public/llms-full.txt is generated from the pages at build time, so it picks
 * up the bumps automatically. (public/llms.txt is a static index with no
 * version snippets.)
 *
 * The snippets show RUST_MSRV with <Version name="RUST_MSRV" />, so an MSRV
 * bump needs no edit in the snippets.
 */

// nym-sdk / nym-bin-common / nym-network-defaults (Rust SDK crates)
export const NYM_SDK_VERSION = "2.0.0";

// smolmix standalone crate
export const SMOLMIX_VERSION = "2.0.0";

// nym-smoldvpn standalone crate
export const SMOLDVPN_VERSION = "2.0.0";

// TypeScript SDK packages (published to npm). mix-fetch is on its own 3.x track
// after the v2 to v3 break; the tunnel + mix-dns + mix-websocket facades share
// a 0.x line for now. Bump these to match the published npm versions.
export const MIX_FETCH_VERSION = "3.0.0";
export const MIX_TUNNEL_VERSION = "0.3.0";
export const MIX_DNS_VERSION = "0.3.0";
export const MIX_WEBSOCKET_VERSION = "0.3.0";

// Minimum supported Rust version (matches workspace rust-version in root Cargo.toml)
export const RUST_MSRV = "1.87";
