[**@nymproject/mix-websocket**](../globals.md) • **Docs**

***

[@nymproject/mix-websocket](../globals.md) / MixWebSocket

# Class: MixWebSocket

WebSocket-like channel over the Nym mixnet. The tunnel must already be
set up (`setupMixTunnel()`) before constructing one.

Differences from the browser `WebSocket`:
  - Constructor resolves asynchronously; use `await ws.opened()` if you
    need to block until the upgrade completes.
  - `binaryType` is fixed to `arraybuffer` (no Blob support).
  - No `bufferedAmount`; the tunnel queues writes through the worker.

## Extends

- `EventTarget`

## Constructors

### new MixWebSocket()

> **new MixWebSocket**(`url`, `protocols`?): [`MixWebSocket`](MixWebSocket.md)

#### Parameters

• **url**: `string`

• **protocols?**: `string` \| `string`[]

#### Returns

[`MixWebSocket`](MixWebSocket.md)

#### Overrides

`EventTarget.constructor`

#### Source

[mix-websocket/src/index.ts:44](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L44)

## Properties

### url

> `readonly` **url**: `string`

#### Source

[mix-websocket/src/index.ts:38](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L38)

***

### protocols

> `readonly` **protocols**: `string`[]

#### Source

[mix-websocket/src/index.ts:39](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L39)

***

### handleIdPromise

> `private` **handleIdPromise**: `Promise`\<`number`\>

#### Source

[mix-websocket/src/index.ts:41](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L41)

***

### state

> `private` **state**: [`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md) = `CONNECTING`

#### Source

[mix-websocket/src/index.ts:42](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L42)

## Accessors

### readyState

> `get` **readyState**(): [`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

#### Returns

[`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

#### Source

[mix-websocket/src/index.ts:76](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L76)

## Methods

### opened()

> **opened**(): `Promise`\<`void`\>

Block until the WebSocket transitions out of `CONNECTING`. Resolves when
`open` fires (or when the connection fails before opening).

#### Returns

`Promise`\<`void`\>

#### Source

[mix-websocket/src/index.ts:84](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L84)

***

### send()

> **send**(`data`): `Promise`\<`void`\>

#### Parameters

• **data**: `string` \| `ArrayBuffer` \| `Uint8Array`

#### Returns

`Promise`\<`void`\>

#### Source

[mix-websocket/src/index.ts:97](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L97)

***

### close()

> **close**(`code`, `reason`): `Promise`\<`void`\>

#### Parameters

• **code**: `number`= `1000`

• **reason**: `string`= `''`

#### Returns

`Promise`\<`void`\>

#### Source

[mix-websocket/src/index.ts:106](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L106)

***

### handleEvent()

> `private` **handleEvent**(`type`, `data`): `void`

#### Parameters

• **type**: `WsEventType`

• **data**: `unknown`

#### Returns

`void`

#### Source

[mix-websocket/src/index.ts:114](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L114)
