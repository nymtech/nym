import crypto from 'node:crypto';
import fs from 'node:fs';
import { TextDecoder, TextEncoder } from 'node:util';
import setGlobalVars from 'indexeddbshim';
import WebSocket from 'ws';
import fetch, {
    Headers,
    Request,
    Response,
} from 'node-fetch'


if (!globalThis.fs) {
    globalThis.fs = fs
}

if (!globalThis.process) {
    globalThis.process = process
}

if (!globalThis.crypto) {
    globalThis.crypto = {
        getRandomValues(b) {
            crypto.randomFillSync(b);
        },
    }
}

if (!globalThis.performance) {
    globalThis.performance = {
        now() {
            const [sec, nsec] = process.hrtime();
            return sec * 1000 + nsec / 1000000;
        },
    };
}

if (!globalThis.TextEncoder) {
    globalThis.TextEncoder = TextEncoder
}

if (!globalThis.TextDecoder) {
    globalThis.TextDecoder = TextDecoder
}

if (!globalThis.fetch) {
    globalThis.fetch = fetch
    globalThis.Headers = Headers
    globalThis.Request = Request
    globalThis.Response = Response
}

// checkOrigin:false is required to avoid  SecurityError Cannot open
// an IndexedDB database from an opaque origin.
setGlobalVars(globalThis, {checkOrigin: false})
globalThis.WebSocket = WebSocket

// has to be loaded after all the polyfill action
import('@nymproject/mix-fetch-wasm/wasm_exec.js')
