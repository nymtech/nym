[**@nymproject/sdk**](../globals.md)

***

[@nymproject/sdk](../globals-1.md) / EventHandlerSubscribeFn

# Type Alias: EventHandlerSubscribeFn\<E\>

> **EventHandlerSubscribeFn**\<`E`\> = (`fn`) => [`EventHandlerUnsubscribeFn`](EventHandlerUnsubscribeFn.md)

Defined in: [mixnet/wasm/types.ts:301](https://github.com/nymtech/nym/blob/develop/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L301)

The **EventHandlerSubscribeFn** is a function that takes a callback of type [EventHandlerFn](EventHandlerFn.md)

## Type Parameters

### E

`E`

## Parameters

### fn

[`EventHandlerFn`](EventHandlerFn.md)\<`E`\>

## Returns

[`EventHandlerUnsubscribeFn`](EventHandlerUnsubscribeFn.md)

## See

 - [Events](../interfaces/Events.md)
 - [EventHandlerFn](EventHandlerFn.md)
 - [EventHandlerUnsubscribeFn](EventHandlerUnsubscribeFn.md)
