/**
 * Centralised version constants for documentation.
 *
 * Components (VersionBanner, CodeVerified, etc.) import from here so there is a
 * single place to update on each release.
 *
 * Fenced code blocks still hardcode versions for copy-paste friendliness.
 * When bumping versions here, also update the Cargo.toml snippets in:
 *
 *   pages/developers/rust/importing.mdx
 *   pages/developers/rust/mixnet/tutorial.mdx
 *   pages/developers/rust/stream/tutorial.mdx
 *   pages/developers/rust/client-pool/tutorial.mdx
 *   pages/developers/smolmix.mdx          (SMOLMIX_VERSION)
 *   pages/developers/smoldvpn.mdx         (SMOLDVPN_VERSION)
 *
 * public/llms-full.txt is generated from these pages by generate-llms-txt.mjs
 * at build time, so it picks up the bumps automatically. (public/llms.txt is a
 * static index with no version snippets.)
 *
 * RUST_MSRV is imported directly by all pages that display the Rust version,
 * so no manual file edits are needed for MSRV bumps:
 *
 *   pages/developers/smolmix.mdx
 *   pages/developers/smoldvpn.mdx
 *   pages/developers/rust/importing.mdx
 */

// nym-sdk / nym-bin-common / nym-network-defaults (Rust SDK crates)
export const NYM_SDK_VERSION = "1.21.5";

// smolmix standalone crate
export const SMOLMIX_VERSION = "1.21.5";

// nym-smoldvpn standalone crate
export const SMOLDVPN_VERSION = "1.21.5";

// TypeScript SDK packages (published to npm). mix-fetch is on its own 2.x track
// after the v1 to v2 break; the tunnel + mix-dns + mix-websocket facades share
// a 0.x line for now. Bump these to match the published npm versions.
export const MIX_FETCH_VERSION = "2.0.0";
export const MIX_TUNNEL_VERSION = "0.1.0";
export const MIX_DNS_VERSION = "0.1.0";
export const MIX_WEBSOCKET_VERSION = "0.1.0";

// Minimum supported Rust version (matches workspace rust-version in root Cargo.toml)
export const RUST_MSRV = "1.87";
