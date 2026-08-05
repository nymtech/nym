[**@nymproject/sdk**](../globals.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / createNymMixnetClient

# Function: createNymMixnetClient()

> **createNymMixnetClient**(`options`?): `Promise`\<[`NymMixnetClient`](../interfaces/NymMixnetClient.md)\>

Create a client to send and receive traffic from the Nym mixnet.

## Parameters

• **options?**: [`NymMixnetClientOptions`](../interfaces/NymMixnetClientOptions.md)

## Returns

`Promise`\<[`NymMixnetClient`](../interfaces/NymMixnetClient.md)\>

## Required

## Example

```typescript
const client = await createNymMixnetClient();
```

## Source

[mixnet/wasm/index.ts:51](https://github.com/nymtech/nym/blob/429390112bf7ca8aee0dbd32a4308d97e7bbbe11/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L51)
