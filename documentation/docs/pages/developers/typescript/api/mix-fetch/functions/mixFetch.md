[**@nymproject/mix-fetch**](../README.md) • **Docs**

***

[@nymproject/mix-fetch](../globals.md) / mixFetch

# Function: mixFetch()

> **mixFetch**(`url`, `args`, `opts`?): `Promise`\<`Response`\>

mixFetch is a drop-in replacement for the standard `fetch` interface.

## Parameters

• **url**: `string`

The URL to fetch from.

• **args**: `any`

Fetch options.

• **opts?**: `any`

Optionally configure mixFetch when it gets created. This only happens once, the first time it gets used.

## Returns

`Promise`\<`Response`\>

## Source

[index.ts:50](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/mix-fetch/src/index.ts#L50)
