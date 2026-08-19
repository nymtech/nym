// THE canonical list of source trees the documentation is indexed against.
//
// This is deliberately at the root of `documentation/` rather than buried in a
// build script, because it decides more than it looks like it does:
//
//   1. `search_code` on the MCP server can only cite a path inside this list, so
//      an agent asking "how is X implemented" gets nothing for anything outside
//      it.
//   2. Docs-vs-code checking can only compare prose against code it can see, so
//      this list is the boundary of what the documentation can be held to.
//
// A crate missing from here fails silently in both directions: no error, just
// answers that quietly do not mention it.
//
// Widening is cheap to write and not cheap to run. Both index files are traced
// into the /api/mcp lambda (see outputFileTracingIncludes in next.config.js) and
// parsed at every cold start, and a cold build re-embeds every newly covered
// file. Add a root because the documentation describes it, not because it exists.
//
// Paths are repo-relative. A root that does not exist on a given checkout is
// skipped rather than failing the build, so a branch that predates a crate still
// builds.
//
// After changing this list: rebuild the index with VOYAGE_API_KEY set, then run
// `scripts/check-mcp-server.sh <deployment>` — its index-coverage group asserts
// that every root here is actually citable.

export const ROOTS = [
  // Standalone crates
  'smolmix',
  'smoldvpn',

  // SDKs and the wasm packages built from them
  'sdk/rust',
  'sdk/typescript/packages',
  'sdk/typescript/examples',
  'sdk/ffi',
  'wasm/smolmix',
  'wasm/client',
  'wasm/zknym-lib',

  // Packet format and the userspace stack the tunnel is built on
  'common/nymsphinx',
  'common/smol-core',

  // Core client internals, behind every "what does the client do" claim
  'common/client-core',
  'common/client-libs',
  'clients/native',
  'clients/socks5',

  // Exit services: what each one sees is a load-bearing claim in the threat model
  'service-providers/ip-packet-router',
  'service-providers/network-requester',

  // Gateway protocol and bandwidth credentials
  'common/gateway-requests',
  'common/credentials',

  // Node implementation, for the operator docs
  'nym-node',
];
