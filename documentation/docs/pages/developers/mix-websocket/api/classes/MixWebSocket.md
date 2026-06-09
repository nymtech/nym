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

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:44](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L44)

## Properties

### url

> `readonly` **url**: `string`

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:38](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L38)

***

### protocols

> `readonly` **protocols**: `string`[]

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:39](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L39)

***

### handleIdPromise

> `private` **handleIdPromise**: `Promise`\<`number`\>

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:41](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L41)

***

### state

> `private` **state**: [`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md) = `CONNECTING`

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:42](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L42)

## Accessors

### readyState

> `get` **readyState**(): [`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

#### Returns

[`MixWebSocketReadyState`](../type-aliases/MixWebSocketReadyState.md)

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:76](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L76)

## Methods

### opened()

> **opened**(): `Promise`\<`void`\>

Block until the WebSocket transitions out of `CONNECTING`. Resolves when
`open` fires (or when the connection fails before opening).

#### Returns

`Promise`\<`void`\>

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:84](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L84)

***

### send()

> **send**(`data`): `Promise`\<`void`\>

#### Parameters

• **data**: `string` \| `ArrayBuffer` \| `Uint8Array`

#### Returns

`Promise`\<`void`\>

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:97](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L97)

***

### close()

> **close**(`code`, `reason`): `Promise`\<`void`\>

#### Parameters

• **code**: `number`= `1000`

• **reason**: `string`= `''`

#### Returns

`Promise`\<`void`\>

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:106](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L106)

***

### handleEvent()

> `private` **handleEvent**(`type`, `data`): `void`

#### Parameters

• **type**: `WsEventType`

• **data**: `unknown`

#### Returns

`void`

#### Source

[dev/work/nym/sdk/typescript/packages/mix-websocket/src/index.ts:114](https://github.com/nymtech/nym/blob/8ea9a230a7d5819511b34aec8d6705e038511ad3/sdk/typescript/packages/mix-websocket/src/index.ts#L114)

***

### addEventListener()

> **addEventListener**(`type`, `callback`, `options`?): `void`

Appends an event listener for events whose type attribute value is type. The callback argument sets the callback that will be invoked when the event is dispatched.

The options argument sets listener-specific options. For compatibility this can be a boolean, in which case the method behaves exactly as if the value was specified as options's capture.

When set to true, options's capture prevents callback from being invoked when the event's eventPhase attribute value is BUBBLING_PHASE. When false (or not present), callback will not be invoked when event's eventPhase attribute value is CAPTURING_PHASE. Either way, callback will be invoked if event's eventPhase attribute value is AT_TARGET.

When set to true, options's passive indicates that the callback will not cancel the event by invoking preventDefault(). This is used to enable performance optimizations described in § 2.8 Observing event listeners.

When set to true, options's once indicates that the callback will only be invoked once after which the event listener will be removed.

If an AbortSignal is passed for options's signal, then the event listener will be removed when signal is aborted.

The event listener is appended to target's event listener list and is not appended if it has the same type, callback, and capture.

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/addEventListener)

#### Parameters

• **type**: `string`

• **callback**: `null` \| `EventListenerOrEventListenerObject`

• **options?**: `boolean` \| `AddEventListenerOptions`

#### Returns

`void`

#### Inherited from

`EventTarget.addEventListener`

#### Source

.local/share/pnpm/global/5/.pnpm/typescript@5.4.5/node\_modules/typescript/lib/lib.dom.d.ts:8256

***

### dispatchEvent()

> **dispatchEvent**(`event`): `boolean`

Dispatches a synthetic event event to target and returns true if either event's cancelable attribute value is false or its preventDefault() method was not invoked, and false otherwise.

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/dispatchEvent)

#### Parameters

• **event**: `Event`

#### Returns

`boolean`

#### Inherited from

`EventTarget.dispatchEvent`

#### Source

.local/share/pnpm/global/5/.pnpm/typescript@5.4.5/node\_modules/typescript/lib/lib.dom.d.ts:8262

***

### removeEventListener()

> **removeEventListener**(`type`, `callback`, `options`?): `void`

Removes the event listener in target's event listener list with the same type, callback, and options.

[MDN Reference](https://developer.mozilla.org/docs/Web/API/EventTarget/removeEventListener)

#### Parameters

• **type**: `string`

• **callback**: `null` \| `EventListenerOrEventListenerObject`

• **options?**: `boolean` \| `EventListenerOptions`

#### Returns

`void`

#### Inherited from

`EventTarget.removeEventListener`

#### Source

.local/share/pnpm/global/5/.pnpm/typescript@5.4.5/node\_modules/typescript/lib/lib.dom.d.ts:8268
