[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / createNymMixnetClient

# Function: createNymMixnetClient()

> **createNymMixnetClient**(`options?`): `Promise`\<[`NymMixnetClient`](../interfaces/NymMixnetClient.md)\>

Defined in: [mixnet/wasm/index.ts:51](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/index.ts#L51)

Create a client to send and receive traffic from the Nym mixnet.

## Parameters

### options?

[`NymMixnetClientOptions`](../interfaces/NymMixnetClientOptions.md)

## Returns

`Promise`\<[`NymMixnetClient`](../interfaces/NymMixnetClient.md)\>

## Required

## Example

```typescript
const client = await createNymMixnetClient();
```
