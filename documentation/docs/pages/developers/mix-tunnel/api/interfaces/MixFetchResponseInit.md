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

[sdk/typescript/packages/mix-tunnel/src/types.ts:87](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L87)

***

### status

> **status**: `number`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:88](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L88)

***

### statusText

> **statusText**: `string`

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:89](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L89)

***

### headers

> **headers**: [`string`, `string`][]

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:90](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L90)
