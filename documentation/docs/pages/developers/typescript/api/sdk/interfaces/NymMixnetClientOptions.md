[**@nymproject/sdk**](../globals.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / NymMixnetClientOptions

# Interface: NymMixnetClientOptions

Options for the Nym mixnet client.

## Example

```typescript
const client = await createNymMixnetClient({
 autoConvertStringMimeTypes: [MimeTypes.ApplicationJson, MimeTypes.TextPlain],
});
```

## Properties

### autoConvertStringMimeTypes?

> `optional` **autoConvertStringMimeTypes**: `string`[] \| [`MimeTypes`](../enumerations/MimeTypes.md)[]

An array of mime types.

#### Source

[mixnet/wasm/index.ts:29](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L29)
