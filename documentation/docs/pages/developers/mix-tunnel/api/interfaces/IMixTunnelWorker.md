[**@nymproject/mix-tunnel**](../globals.md)

***

[@nymproject/mix-tunnel](../globals-1.md) / IMixTunnelWorker

# Interface: IMixTunnelWorker

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:71](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L71)

## Methods

### setupMixTunnel()

> **setupMixTunnel**(`opts?`): `Promise`\<`void`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:72](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L72)

#### Parameters

##### opts?

[`SetupMixTunnelOpts`](SetupMixTunnelOpts.md)

#### Returns

`Promise`\<`void`\>

***

### disconnectMixTunnel()

> **disconnectMixTunnel**(): `Promise`\<`void`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:73](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L73)

#### Returns

`Promise`\<`void`\>

***

### getTunnelState()

> **getTunnelState**(): `Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:74](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L74)

#### Returns

`Promise`\<[`TunnelState`](../type-aliases/TunnelState.md)\>

***

### mixFetch()

> **mixFetch**(`url`, `init`): `Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:75](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L75)

#### Parameters

##### url

`string`

##### init

`unknown`

#### Returns

`Promise`\<[`MixFetchResponseInit`](MixFetchResponseInit.md)\>

***

### mixDNS()

> **mixDNS**(`hostname`): `Promise`\<`string`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:76](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L76)

#### Parameters

##### hostname

`string`

#### Returns

`Promise`\<`string`\>

***

### mixWebSocket()

> **mixWebSocket**(`url`, `protocols`, `onEvent`): `Promise`\<`number`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:77](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L77)

#### Parameters

##### url

`string`

##### protocols

`string`[] \| `undefined`

##### onEvent

[`WsEventCallback`](../type-aliases/WsEventCallback.md)

#### Returns

`Promise`\<`number`\>

***

### wsSend()

> **wsSend**(`handleId`, `data`): `Promise`\<`void`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:78](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L78)

#### Parameters

##### handleId

`number`

##### data

`string` \| `ArrayBuffer` \| `Uint8Array`\<`ArrayBufferLike`\>

#### Returns

`Promise`\<`void`\>

***

### wsClose()

> **wsClose**(`handleId`, `code`, `reason`): `Promise`\<`void`\>

Defined in: [sdk/typescript/packages/mix-tunnel/src/types.ts:79](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-tunnel/src/types.ts#L79)

#### Parameters

##### handleId

`number`

##### code

`number`

##### reason

`string`

#### Returns

`Promise`\<`void`\>
