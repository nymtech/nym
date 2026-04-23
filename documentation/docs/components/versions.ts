/**
 * Centralised version constants for documentation.
 *
 * Components (CratesPaused, CodeVerified, etc.) import from here so there is a
 * single place to update on each release.
 *
 * Fenced code blocks in tutorials still hardcode versions for copy-paste
 * friendliness.  When bumping versions here, also update the Cargo.toml
 * snippets in the files listed below:
 *
 *   pages/developers/smolmix/tutorial.mdx
 *   pages/developers/smolmix/tutorial-udp.mdx
 *   pages/developers/smolmix/tutorial-websocket.mdx
 *   pages/developers/rust/importing.mdx
 *   pages/developers/rust/mixnet/tutorial.mdx
 *   pages/developers/rust/stream/tutorial.mdx
 *   pages/developers/rust/tcpproxy.mdx
 *   pages/developers/rust/client-pool/tutorial.mdx
 *   public/llms.txt
 */

// nym-sdk / nym-bin-common / nym-network-defaults — Rust SDK crates
export const NYM_SDK_VERSION = "X.Y.Z";

// smolmix standalone crate
export const SMOLMIX_VERSION = "X.Y.Z";

// blake3 exact pin (workaround for transitive digest conflict)
export const BLAKE3_PIN = "=1.7.0";
