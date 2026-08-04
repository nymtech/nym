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

[types.ts:72](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L72)

***

### disconnectMixTunnel()

> **disconnectMixTunnel**(): `Promise`\<`void`\>

#### Returns

`Promise`\<`void`\>

#### Source

[types.ts:73](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L73)

***

### getTunnelState()

> **getTunnelState**(): `Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Returns

`Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

#### Source

[types.ts:74](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L74)

***

### mixFetch()

> **mixFetch**(`url`, `init`): `Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Parameters

• **url**: `string`

• **init**: `unknown`

#### Returns

`Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

#### Source

[types.ts:75](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L75)

***

### mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

#### Parameters

• **hostname**: `string`

#### Returns

`Promise`\<`string`\>

#### Source

[types.ts:76](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L76)

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

[types.ts:77](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L77)

***

### wsSend()

> **wsSend**(`handleId`, `data`): `Promise`\<`void`\>

#### Parameters

• **handleId**: `number`

• **data**: `string` \| `Uint8Array` \| `ArrayBuffer`

#### Returns

`Promise`\<`void`\>

#### Source

[types.ts:78](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L78)

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

[types.ts:79](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/mix-tunnel/src/types.ts#L79)
