[**@nymproject/mix-tunnel**](../globals.md) • **Docs**

***

[@nymproject/mix-tunnel](../globals.md) / getMixTunnel

# Function: getMixTunnel()

> **getMixTunnel**(): `Promise`\<`Remote`\<[`IMixTunnelWorker`](../interfaces/IMixTunnelWorker.md)\>\>

Get the singleton tunnel worker handle. The first call spawns the worker
and loads smolmix-wasm; subsequent calls return the same handle.

Note: this does NOT call `setupMixTunnel` automatically. Call it on the
returned handle (or use the top-level `setupMixTunnel` helper) before
issuing fetch/dns/websocket requests.

## Returns

`Promise`\<`Remote`\<[`IMixTunnelWorker`](../interfaces/IMixTunnelWorker.md)\>\>

## Source

[index.ts:47](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/index.ts#L47)
