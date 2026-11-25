// Citability probes for the code index, one per path the live check asserts a hit
// under. `lib/retrieval/index-coverage.test.ts` fails unless every entry in ROOTS
// (indexed-sources.mjs) has a probe at or under it, and `check-mcp-server.sh`
// runs each probe against a deployment.
//
// This file sits in scripts/, outside the /api/mcp lambda trace, so the queries
// never ship to production.
//
// Prefer symbols over prose. A description of what a crate does competes with
// every other crate that does something similar, and the field keeps growing: an
// earlier query using "chunk" lost to common/nymsphinx/chunking, and its
// replacement, "checkpoint snapping and start jitter to obfuscate a resume
// height", later lost to smoldvpn/examples/zcash-sync.rs once that arrived. Both
// times the crate was fully indexed and the query had simply stopped
// discriminating. A symbol only one crate defines does not decay that way.

export const PROBES = {
  smolmix: {
    label: 'smolmix: the userspace TCP/UDP tunnel',
    query: 'Tunnel new_with_ipr get_best_ipr from_stream',
  },
  smoldvpn: {
    label: 'smoldvpn: the WireGuard datapath',
    query: 'wireguard peer configuration and tunnel engine setup',
  },
  'sdk/rust/nym-swizzle': {
    label: 'sdk/rust: nym-swizzle start obfuscation',
    query: 'obfuscated_start snap grid floor jitter',
  },
  'sdk/typescript/packages': {
    label: 'sdk/typescript/packages: the mix-fetch and mix-tunnel packages',
    query: 'createMixFetch createTunnel getMixTunnel',
  },
  'sdk/typescript/examples': {
    label: 'sdk/typescript/examples: the browser examples',
    query: 'createChatEntry appendOutput createISOTimeStamp',
  },
  'sdk/ffi': {
    label: 'sdk/ffi: the Go and C++ bindings',
    query: 'RustBufferFromExternal FfiRustBufConverter go-nym bindings',
  },
  'wasm/smolmix': {
    label: 'wasm/smolmix: the wasm tunnel device',
    query: 'WasmDevice parse_outbound_tcp WasmTxToken',
  },
  'wasm/client': {
    label: 'wasm/client: the wasm mixnet client',
    query: 'ResponsePusher EncodedPayloadMetadata parse_payload',
  },
  'wasm/zknym-lib': {
    label: 'wasm/zknym-lib: zk-nym ticketbooks in wasm',
    query: 'SerialisedNymTicketBook NymVpnApiClient',
  },
  'common/nymsphinx': {
    label: 'common/nymsphinx: packet construction',
    query: 'sphinx packet header construction and layer encryption',
  },
  'common/smol-core': {
    label: 'common/smol-core: the userspace network stack',
    query: 'ChannelDevice client_mtu StackConfig SmolCoreError',
  },
  'common/client-core': {
    label: 'common/client-core: the client internals',
    query: 'client topology refresh and gateway selection',
  },
  'common/client-libs': {
    label: 'common/client-libs: the gateway and mixnet clients',
    query: 'BandwidthClaimGuard connect_async_with_hickory',
  },
  'clients/native': {
    label: 'clients/native: the native websocket client',
    query: 'native client websocket handler ClientRequest SelfAddress',
  },
  'clients/socks5': {
    label: 'clients/socks5: the socks5 client',
    query: 'CliSocks5Client socks5 client listener accepting a connection',
  },
  'service-providers/ip-packet-router': {
    label: 'service-providers: the IP packet router',
    query: 'ip packet router exit policy and tunnel address allocation',
  },
  'service-providers/network-requester': {
    label: 'service-providers: the network requester',
    query: 'network requester socks5 connect request handling',
  },
  'common/gateway-requests': {
    label: 'common/gateway-requests: the gateway protocol',
    query: 'gateway client authentication and registration handshake',
  },
  'common/credentials': {
    label: 'common/credentials: bandwidth credentials',
    query: 'bandwidth credential issuance and verification',
  },
  'nym-node': {
    label: 'nym-node: the node implementation',
    query: 'nym-node configuration and mode selection on startup',
  },
};
