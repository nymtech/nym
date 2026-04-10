[**@nymproject/sdk**](../globals.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / Events

# Interface: Events

## Properties

### subscribeToLoaded

> **subscribeToLoaded**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`LoadedEvent`](LoadedEvent.md)\>

#### See

[LoadedEvent](LoadedEvent.md)

#### Example

```typescript
events.subscribeToLoaded((e) => {
 console.log(e.args); // { loaded: true }
});
```

#### Source

[mixnet/wasm/types.ts:143](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L143)

***

### subscribeToConnected

> **subscribeToConnected**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`ConnectedEvent`](ConnectedEvent.md)\>

#### See

[ConnectedEvent](ConnectedEvent.md)

#### Example

```typescript
events.subscribeConnected((e) => {
 console.log(e.args.address); // Client address
});

#### Source

[mixnet/wasm/types.ts:153](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L153)

***

### subscribeToTextMessageReceivedEvent

> **subscribeToTextMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`StringMessageReceivedEvent`](StringMessageReceivedEvent.md)\>

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

#### Source

[mixnet/wasm/types.ts:167](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L167)

***

### subscribeToBinaryMessageReceivedEvent

> **subscribeToBinaryMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`BinaryMessageReceivedEvent`](BinaryMessageReceivedEvent.md)\>

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

#### Source

[mixnet/wasm/types.ts:181](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L181)

***

### subscribeToRawMessageReceivedEvent

> **subscribeToRawMessageReceivedEvent**: [`EventHandlerSubscribeFn`](../type-aliases/EventHandlerSubscribeFn.md)\<[`RawMessageReceivedEvent`](RawMessageReceivedEvent.md)\>

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

#### Source

[mixnet/wasm/types.ts:195](https://github.com/nymtech/nym/blob/5065c5579e2c961211276dcdfd8aaa127f29bf26/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L195)
