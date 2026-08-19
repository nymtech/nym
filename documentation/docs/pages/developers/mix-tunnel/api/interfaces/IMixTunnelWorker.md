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

[sdk/typescript/packages/mix-tunnel/src/types.ts:72](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L72)

***

### disconnectMixTunnel()

> **disconnectMixTunnel**(): `Promise`\<`void`\>

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:73](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L73)

***

### getTunnelState()

> **getTunnelState**(): `Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Returns

`Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:74](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L74)

***

### mixFetch()

> **mixFetch**(`url`, `init`): `Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Parameters

• **url**: `string`

• **init**: `unknown`

#### Returns

`Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:75](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L75)

***

### mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

#### Parameters

• **hostname**: `string`

#### Returns

`Promise`\<`string`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:76](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L76)

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

[sdk/typescript/packages/mix-tunnel/src/types.ts:77](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L77)

***

### wsSend()

> **wsSend**(`handleId`, `data`): `Promise`\<`void`\>

#### Parameters

• **handleId**: `number`

• **data**: `string` \| `Uint8Array` \| `ArrayBuffer`

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:78](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L78)

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

[sdk/typescript/packages/mix-tunnel/src/types.ts:79](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L79)
