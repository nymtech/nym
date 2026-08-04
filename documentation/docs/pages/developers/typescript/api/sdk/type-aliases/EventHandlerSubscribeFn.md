[**@nymproject/sdk**](../globals.md) • **Docs**

***

[@nymproject/sdk](../globals.md) / EventHandlerSubscribeFn

# Type alias: EventHandlerSubscribeFn()\<E\>

> **EventHandlerSubscribeFn**\<`E`\>: (`fn`) => [`EventHandlerUnsubscribeFn`](EventHandlerUnsubscribeFn.md)

The **EventHandlerSubscribeFn** is a function that takes a callback of type [EventHandlerFn](EventHandlerFn.md)

## See

 - [Events](../interfaces/Events.md)
 - [EventHandlerFn](EventHandlerFn.md)
 - [EventHandlerUnsubscribeFn](EventHandlerUnsubscribeFn.md)

## Type parameters

• **E**

## Parameters

• **fn**: [`EventHandlerFn`](EventHandlerFn.md)\<`E`\>

## Returns

[`EventHandlerUnsubscribeFn`](EventHandlerUnsubscribeFn.md)

## Source

[mixnet/wasm/types.ts:301](https://github.com/nymtech/nym/blob/c21c8241da3fe141f1e4a0c9a6a2597bce1a064c/sdk/typescript/packages/sdk/src/mixnet/wasm/types.ts#L301)
