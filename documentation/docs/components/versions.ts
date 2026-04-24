/**
 * Centralised version constants for documentation.
 *
 * Components (CratesPaused, CodeVerified, etc.) import from here so there is a
 * single place to update on each release.
 *
 * Fenced code blocks still hardcode versions for copy-paste friendliness.
 * When bumping versions here, also update the Cargo.toml snippets in:
 *
 *   pages/developers/rust/importing.mdx
 *   pages/developers/rust/tcpproxy.mdx
 *   public/llms.txt
 *
 * RUST_MSRV is imported directly by all pages that display the Rust version —
 * no manual file edits needed for MSRV bumps:
 *
 *   pages/developers/smolmix.mdx
 *   pages/developers/rust/importing.mdx
 */

// nym-sdk / nym-bin-common / nym-network-defaults — Rust SDK crates
export const NYM_SDK_VERSION = "1.20.4";

// smolmix standalone crate
export const SMOLMIX_VERSION = "0.0.1";

// blake3 exact pin (workaround for transitive digest conflict)
export const BLAKE3_PIN = "=1.7.0";

// Minimum supported Rust version (matches workspace rust-version in root Cargo.toml)
export const RUST_MSRV = "1.87";
