[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / NymMixnetClientOptions

# Interface: NymMixnetClientOptions

Defined in: [mixnet/wasm/index.ts:28](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L28)

Options for the Nym mixnet client.

## Example

```typescript
const client = await createNymMixnetClient({
 autoConvertStringMimeTypes: [MimeTypes.ApplicationJson, MimeTypes.TextPlain],
});
```

## Properties

### autoConvertStringMimeTypes?

> `optional` **autoConvertStringMimeTypes?**: `string`[] \| [`MimeTypes`](../enumerations/MimeTypes.md)[]

Defined in: [mixnet/wasm/index.ts:29](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L29)

An array of mime types.
