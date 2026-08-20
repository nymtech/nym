[**@nymproject/mix-tunnel**](../globals.md)

***

[@nymproject/mix-tunnel](../globals-1.md) / proxy

# Function: proxy()

> **proxy**\<`T`\>(`obj`): `T` & `ProxyMarked`

Defined in: node\_modules/.pnpm/comlink@4.4.2/node\_modules/comlink/dist/umd/comlink.d.ts:154

Re-export of `Comlink.proxy` so feature packages (mix-websocket etc.) can
mark callbacks for proxy-transfer using THIS module's Comlink instance.

Why: Comlink detects proxy-marked values via a `Symbol('Comlink.proxy')`.
That symbol is created per module instance, so if mix-websocket bundled its
own Comlink, `mix-websocket.Comlink.proxy(fn)` would mark `fn` with a
symbol that mix-tunnel's serializer doesn't recognise, falling through to
structured-clone, which can't clone functions, throws DOMException.

By exposing this re-export, mix-websocket's `import { proxy } from
'@nymproject/mix-tunnel'` returns the same function backed by the same
Comlink instance, so the marker symbol matches.

## Type Parameters

### T

`T` *extends* `object`

## Parameters

### obj

`T`

## Returns

`T` & `ProxyMarked`
