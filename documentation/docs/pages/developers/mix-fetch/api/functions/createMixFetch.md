[**@nymproject/mix-fetch**](../globals.md) • **Docs**

***

[@nymproject/mix-fetch](../globals.md) / createMixFetch

# Function: createMixFetch()

> **createMixFetch**(`opts`?): `Promise`\<(`url`, `init`?) => `Promise`\<`Response`\>\>

Convenience: set up the tunnel and return a fetch-bound function. Equivalent
to `await setupMixTunnel(opts); return mixFetch;`. Safe to call multiple
times; the underlying tunnel is a singleton.

## Parameters

• **opts?**: [`SetupMixTunnelOpts`](../interfaces/SetupMixTunnelOpts.md)

## Returns

`Promise`\<(`url`, `init`?) => `Promise`\<`Response`\>\>

## Source

[mix-fetch/src/index.ts:62](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-fetch/src/index.ts#L62)
