[**@nymproject/mix-websocket**](../globals.md)

***

[@nymproject/mix-websocket](../globals-1.md) / MixWebSocket

# Class: MixWebSocket

Defined in: [src/index.ts:37](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L37)

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

### Constructor

> **new MixWebSocket**(`url`, `protocols?`): `MixWebSocket`

Defined in: [src/index.ts:44](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L44)

#### Parameters

##### url

`string`

##### protocols?

`string` \| `string`[]

#### Returns

`MixWebSocket`

#### Overrides

`EventTarget.constructor`

## Properties

### url

> `readonly` **url**: `string`

Defined in: [src/index.ts:38](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L38)

***

### protocols

> `readonly` **protocols**: `string`[]

Defined in: [src/index.ts:39](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L39)

## Accessors

### readyState

#### Get Signature

> **get** **readyState**(): [`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

Defined in: [src/index.ts:76](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L76)

##### Returns

[`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

## Methods

### opened()

> **opened**(): `Promise`\<`void`\>

Defined in: [src/index.ts:84](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L84)

Block until the WebSocket transitions out of `CONNECTING`. Resolves when
`open` fires (or when the connection fails before opening).

#### Returns

`Promise`\<`void`\>

***

### send()

> **send**(`data`): `Promise`\<`void`\>

Defined in: [src/index.ts:97](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L97)

#### Parameters

##### data

`string` \| `ArrayBuffer` \| `Uint8Array`\<`ArrayBufferLike`\>

#### Returns

`Promise`\<`void`\>

***

### close()

> **close**(`code?`, `reason?`): `Promise`\<`void`\>

Defined in: [src/index.ts:106](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/mix-websocket/src/index.ts#L106)

#### Parameters

##### code?

`number` = `1000`

##### reason?

`string` = `''`

#### Returns

`Promise`\<`void`\>

***

### addEventListener()

> **addEventListener**(`type`, `callback`, `options?`): `void`

Defined in: ../../../../../../../../opt/homebrew/lib/node\_modules/typedoc/node\_modules/typescript/lib/lib.dom.d.ts:14380

The **`addEventListener()`** method of the EventTarget interface sets up a function that will be called whenever the specified event is delivered to the target.

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/addEventListener)

#### Parameters

##### type

`string`

##### callback

`EventListenerOrEventListenerObject` \| `null`

##### options?

`boolean` \| `AddEventListenerOptions`

#### Returns

`void`

#### Inherited from

`EventTarget.addEventListener`

***

### dispatchEvent()

> **dispatchEvent**(`event`): `boolean`

Defined in: ../../../../../../../../opt/homebrew/lib/node\_modules/typedoc/node\_modules/typescript/lib/lib.dom.d.ts:14386

The **`dispatchEvent()`** method of the EventTarget sends an Event to the object, (synchronously) invoking the affected event listeners in the appropriate order. The normal event processing rules (including the capturing and optional bubbling phase) also apply to events dispatched manually with dispatchEvent().

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/dispatchEvent)

#### Parameters

##### event

`Event`

#### Returns

`boolean`

#### Inherited from

`EventTarget.dispatchEvent`

***

### removeEventListener()

> **removeEventListener**(`type`, `callback`, `options?`): `void`

Defined in: ../../../../../../../../opt/homebrew/lib/node\_modules/typedoc/node\_modules/typescript/lib/lib.dom.d.ts:14392

The **`removeEventListener()`** method of the EventTarget interface removes an event listener previously registered with EventTarget.addEventListener() from the target. The event listener to be removed is identified using a combination of the event type, the event listener function itself, and various optional options that may affect the matching process; see Matching event listeners for removal.

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/removeEventListener)

#### Parameters

##### type

`string`

##### callback

`EventListenerOrEventListenerObject` \| `null`

##### options?

`boolean` \| `EventListenerOptions`

#### Returns

`void`

#### Inherited from

`EventTarget.removeEventListener`
