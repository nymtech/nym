[**@nymproject/mix-fetch**](../globals.md)

***

[@nymproject/mix-fetch](../globals-1.md) / mixFetch

# Function: mixFetch()

> **mixFetch**(`url`, `init?`): `Promise`\<`Response`\>

Defined in: [mix-fetch/src/index.ts:39](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-fetch/src/index.ts#L39)

Fetch over the mixnet. Drop-in replacement for the browser `fetch()`.

Requires the tunnel to be up: call `setupMixTunnel(opts)` first, or use
`createMixFetch(opts)` to combine setup + fetch.

## Parameters

### url

`string`

### init?

`RequestInit`

## Returns

`Promise`\<`Response`\>
