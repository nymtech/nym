[**@nymproject/mix-fetch**](../globals.md) • **Docs**

***

[@nymproject/mix-fetch](../globals.md) / mixFetch

# Function: mixFetch()

> **mixFetch**(`url`, `init`?): `Promise`\<`Response`\>

Fetch over the mixnet. Drop-in replacement for the browser `fetch()`.

Requires the tunnel to be up: call `setupMixTunnel(opts)` first, or use
`createMixFetch(opts)` to combine setup + fetch.

## Parameters

• **url**: `string`

• **init?**: `RequestInit`

## Returns

`Promise`\<`Response`\>

## Source

[mix-fetch/src/index.ts:39](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-fetch/src/index.ts#L39)
