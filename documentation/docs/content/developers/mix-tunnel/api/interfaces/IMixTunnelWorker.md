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

[sdk/typescript/packages/mix-tunnel/src/types.ts:101](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L101)

***

### disconnectMixTunnel()

> **disconnectMixTunnel**(): `Promise`\<`void`\>

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:102](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L102)

***

### getTunnelState()

> **getTunnelState**(): `Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Returns

`Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:103](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L103)

***

### mixFetch()

> **mixFetch**(`url`, `init`): `Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Parameters

• **url**: `string`

• **init**: `unknown`

#### Returns

`Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:104](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L104)

***

### mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

#### Parameters

• **hostname**: `string`

#### Returns

`Promise`\<`string`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:105](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L105)

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

[sdk/typescript/packages/mix-tunnel/src/types.ts:106](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L106)

***

### wsSend()

> **wsSend**(`handleId`, `data`): `Promise`\<`void`\>

#### Parameters

• **handleId**: `number`

• **data**: `string` \| `Uint8Array` \| `ArrayBuffer`

#### Returns

`Promise`\<`void`\>

#### Source

[sdk/typescript/packages/mix-tunnel/src/types.ts:107](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L107)

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

[sdk/typescript/packages/mix-tunnel/src/types.ts:108](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L108)
