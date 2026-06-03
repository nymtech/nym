[**@nymproject/mix-tunnel**](../globals.md) • **Docs**

***

[@nymproject/mix-tunnel](../globals.md) / MixFetchResponseInit

# Interface: MixFetchResponseInit

Pre-serialised response shape produced by `smolmix-wasm::mixFetch`. Designed
for Comlink transfer (Uint8Array + primitive arrays survive structured clone).

`headers` is a sequence of `[name, value]` pairs rather than a record so that
repeated names like `Set-Cookie`, `Vary`, `Link`, `WWW-Authenticate` survive.
The TS facade reconstructs a real `Response` via:

  new Response(raw.body, {
    status: raw.status,
    statusText: raw.statusText,
    headers: new Headers(raw.headers),
  })

## Properties

### body

> **body**: `Uint8Array`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:51](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L51)

***

### status

> **status**: `number`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:52](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L52)

***

### statusText

> **statusText**: `string`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:53](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L53)

***

### headers

> **headers**: [`string`, `string`][]

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:54](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L54)
