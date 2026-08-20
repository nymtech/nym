[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / Events

# Interface: Events

Defined in: [mixnet/wasm/types.ts:133](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L133)

## Properties

### subscribeToLoaded

> **subscribeToLoaded**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`LoadedEvent`](LoadedEvent.md)\>

Defined in: [mixnet/wasm/types.ts:143](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L143)

#### See

[LoadedEvent](LoadedEvent.md)

#### Example

```typescript
events.subscribeToLoaded((e) => {
 console.log(e.args); // { loaded: true }
});
```

***

### subscribeToConnected

> **subscribeToConnected**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`ConnectedEvent`](ConnectedEvent.md)\>

Defined in: [mixnet/wasm/types.ts:153](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L153)

#### See

[ConnectedEvent](ConnectedEvent.md)

#### Example

```typescript
events.subscribeConnected((e) => {
 console.log(e.args.address); // Client address
});

***

### subscribeToTextMessageReceivedEvent

> **subscribeToTextMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`StringMessageReceivedEvent`](StringMessageReceivedEvent.md)\>

Defined in: [mixnet/wasm/types.ts:167](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L167)

#### Returns

[EventHandlerUnsubscribeFn](../type-aliases/EventHandlerUnsubscribeFn.md)

#### See

[StringMessageReceivedEvent](StringMessageReceivedEvent.md)

#### Example

```typescript
const unsubscribe = events.subscribeToTextMessageReceivedEvent((e) => {
 console.log(e.args.payload); // string
});

// Stop listening to the event
unsubscribe();
```

***

### subscribeToBinaryMessageReceivedEvent

> **subscribeToBinaryMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`BinaryMessageReceivedEvent`](BinaryMessageReceivedEvent.md)\>

Defined in: [mixnet/wasm/types.ts:181](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L181)

#### Returns

[EventHandlerUnsubscribeFn](../type-aliases/EventHandlerUnsubscribeFn.md)

#### See

[BinaryMessageReceivedEvent](BinaryMessageReceivedEvent.md)

#### Example

```typescript
const unsubscribe = events.subscribeToBinaryMessageReceivedEvent((e) => {
 console.log(e.args.payload); // Uint8Array
});

// Stop listening to the event
unsubscribe();
```

***

### subscribeToRawMessageReceivedEvent

> **subscribeToRawMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`RawMessageReceivedEvent`](RawMessageReceivedEvent.md)\>

Defined in: [mixnet/wasm/types.ts:195](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L195)

#### Returns

[EventHandlerUnsubscribeFn](../type-aliases/EventHandlerUnsubscribeFn.md)

#### See

[RawMessageReceivedEvent](RawMessageReceivedEvent.md)

#### Example

```typescript
const unsubscribe = events.subscribeToRawMessageReceivedEvent((e) => {
 console.log(e.args.payload); // Uint8Array
});

// Stop listening to the event
unsubscribe();
```
