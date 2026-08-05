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

[mixnet/wasm/index.ts:29](https://github.com/nymtech/nym/blob/429390112bf7ca8aee0dbd32a4308d97e7bbbe11/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L29)
