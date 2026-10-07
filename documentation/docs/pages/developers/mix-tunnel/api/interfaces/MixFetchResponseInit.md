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

[sdk/typescript/packages/mix-tunnel/src/types.ts:91](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L91)

***

### status

> **status**: `number`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:92](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L92)

***

### statusText

> **statusText**: `string`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:93](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L93)

***

### headers

> **headers**: [`string`, `string`][]

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:94](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L94)
