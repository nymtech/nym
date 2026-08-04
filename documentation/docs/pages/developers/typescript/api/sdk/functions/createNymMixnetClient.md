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

[mixnet/wasm/index.ts:51](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L51)
