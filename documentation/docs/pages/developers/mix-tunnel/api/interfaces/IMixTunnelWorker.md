[**@nymproject/mix-tunnel**](../globals.md) • **Docs**

***

[@nymproject/mix-tunnel](../globals.md) / IMixTunnelWorker

# Interface: IMixTunnelWorker

## Methods

### setupMixTunnel()

> **setupMixTunnel**(`opts`?): `Promise`\<`void`\>

#### Parameters

• **opts?**: [`SetupMixTunnelOpts`](SetupMixTunnelOpts.md)

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:61](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L61)

***

### disconnectMixTunnel()

> **disconnectMixTunnel**(): `Promise`\<`void`\>

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:62](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L62)

***

### getTunnelState()

> **getTunnelState**(): `Promise`\<[`TunnelState`](TunnelState.md)\>

#### Returns

`Promise`\<[`TunnelState`](TunnelState.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:63](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L63)

***

### mixFetch()

> **mixFetch**(`url`, `init`): `Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Parameters

• **url**: `string`

• **init**: `unknown`

#### Returns

`Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:64](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L64)

***

### mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

#### Parameters

• **hostname**: `string`

#### Returns

`Promise`\<`string`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:65](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L65)

***

### mixWebSocket()

> **mixWebSocket**(`url`, `protocols`, `onEvent`): `Promise`\<`number`\>

#### Parameters

• **url**: `string`

• **protocols**: `undefined` \| `string`[]

• **onEvent**: [`WsEventCallback`](../type-aliases/WsEventCallback.md)

#### Returns

`Promise`\<`number`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:66](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L66)

***

### wsSend()

> **wsSend**(`handleId`, `data`): `Promise`\<`void`\>

#### Parameters

• **handleId**: `number`

• **data**: `string` \| `Uint8Array` \| `ArrayBuffer`

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:67](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L67)

***

### wsClose()

> **wsClose**(`handleId`, `code`, `reason`): `Promise`\<`void`\>

#### Parameters

• **handleId**: `number`

• **code**: `number`

• **reason**: `string`

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:68](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-tunnel/src/types.ts#L68)
