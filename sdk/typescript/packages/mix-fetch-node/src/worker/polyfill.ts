import { TextDecoder, TextEncoder } from 'node:util';
import * as crypto from 'node:crypto';
import * as fs from 'node:fs';
import WebSocket from 'ws';
import fetch, { Headers, Request, Response } from 'node-fetch';
import { Worker } from 'node:worker_threads';
import { indexedDB } from 'fake-indexeddb';

(globalThis as any).performance = {
  now() {
    const [sec, nsec] = process.hrtime();
    return sec * 1000 + nsec / 1000000;
  },
};

(globalThis as any).TextDecoder = TextDecoder;
(globalThis as any).fetch = fetch;
(globalThis as any).Headers = Headers;
(globalThis as any).Request = Request;
(globalThis as any).Response = Response;
(globalThis as any).fs = fs;
(globalThis as any).crypto = crypto;
(globalThis as any).WebSocket = WebSocket;
(globalThis as any).Worker = Worker;

globalThis.process = process;
globalThis.TextEncoder = TextEncoder;
globalThis.Reflect = Reflect;
globalThis.Proxy = Proxy;
globalThis.Error = Error;
globalThis.Promise = Promise;
globalThis.Object = Object;
globalThis.indexedDB = indexedDB;

// has to be loaded after all the polyfill action
// eslint-disable-next-line import/extensions, import/no-extraneous-dependencies
import('@nymproject/mix-fetch-wasm-node/wasm_exec');
