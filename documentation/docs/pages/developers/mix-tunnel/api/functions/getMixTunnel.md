[**@nymproject/mix-tunnel**](../globals.md)

***

[@nymproject/mix-tunnel](../globals-1.md) / getMixTunnel

# Function: getMixTunnel()

> **getMixTunnel**(): `Promise`\<`Remote`\<[`IMixTunnelWorker`](../interfaces/IMixTunnelWorker.md)\>\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/index.ts:47](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/index.ts#L47)

Get the singleton tunnel worker handle. The first call spawns the worker
and loads smolmix-wasm; subsequent calls return the same handle.

Note: this does NOT call `setupMixTunnel` automatically. Call it on the
returned handle (or use the top-level `setupMixTunnel` helper) before
issuing fetch/dns/websocket requests.

## Returns

`Promise`\<`Remote`\<[`IMixTunnelWorker`](../interfaces/IMixTunnelWorker.md)\>\>
