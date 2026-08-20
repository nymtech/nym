[**@nymproject/mix-fetch**](../globals.md)

***

[@nymproject/mix-fetch](../globals-1.md) / createMixFetch

# Function: createMixFetch()

> **createMixFetch**(`opts?`): `Promise`\<(`url`, `init?`) => `Promise`\<`Response`\>\>

Defined in: [mix-fetch/src/index.ts:62](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-fetch/src/index.ts#L62)

Convenience: set up the tunnel and return a fetch-bound function. Equivalent
to `await setupMixTunnel(opts); return mixFetch;`. Safe to call multiple
times; the underlying tunnel is a singleton.

## Parameters

### opts?

[`SetupMixTunnelOpts`](../interfaces/SetupMixTunnelOpts.md)

## Returns

`Promise`\<(`url`, `init?`) => `Promise`\<`Response`\>\>
