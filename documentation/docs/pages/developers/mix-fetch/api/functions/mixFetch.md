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

[mix-fetch/src/index.ts:39](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-fetch/src/index.ts#L39)
