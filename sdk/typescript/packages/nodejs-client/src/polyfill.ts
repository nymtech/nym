import * as crypto from 'node:crypto';
import * as fs from 'node:fs';
import ws from 'ws';
import { TextDecoder, TextEncoder } from 'node:util';
import { Worker } from 'node:worker_threads';
import { indexedDB } from 'fake-indexeddb';
import { performance } from 'node:perf_hooks';

(globalThis as any).performance = performance;
(globalThis as any).TextDecoder = TextDecoder;
(globalThis as any).fs = fs;
(globalThis as any).crypto = crypto;
(globalThis as any).ws = ws;
(globalThis as any).Worker = Worker;

globalThis.process = process;
globalThis.TextEncoder = TextEncoder;
globalThis.Reflect = Reflect;
globalThis.Proxy = Proxy;
globalThis.Error = Error;
globalThis.Promise = Promise;
globalThis.Object = Object;
globalThis.indexedDB = indexedDB;
