import * as wasm from './nym_client_wasm_bg.wasm';

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let WASM_VECTOR_LEN = 0;

let cachedUint8Memory0 = new Uint8Array();

function getUint8Memory0() {
    if (cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = new Int32Array();

function getInt32Memory0() {
    if (cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let cachedFloat64Memory0 = new Float64Array();

function getFloat64Memory0() {
    if (cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

let cachedBigInt64Memory0 = new BigInt64Array();

function getBigInt64Memory0() {
    if (cachedBigInt64Memory0.byteLength === 0) {
        cachedBigInt64Memory0 = new BigInt64Array(wasm.memory.buffer);
    }
    return cachedBigInt64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}
function __wbg_adapter_46(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__ha1e005bc9b57c90c(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wbg_adapter_49(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h71a148875a2c330f(arg0, arg1);
}

function __wbg_adapter_52(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h07195fa7991e1ddf(arg0, arg1, addHeapObject(arg2));
}

function makeClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b);
                state.a = 0;

            }
        }
    };
    real.original = state;

    return real;
}
function __wbg_adapter_61(arg0, arg1) {
    wasm._dyn_core__ops__function__Fn_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hada7de604acd93c9(arg0, arg1);
}

function __wbg_adapter_64(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__Fn__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h844be727de79a10b(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_67(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h62aac500ae55364d(arg0, arg1);
}

function __wbg_adapter_70(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__ha67d0876201d9695(arg0, arg1, addHeapObject(arg2));
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
    return instance.ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1);
    getUint8Memory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
* @param {string} api_server
* @param {string | undefined} preferred
* @returns {Promise<GatewayEndpointConfig>}
*/
export function get_gateway(api_server, preferred) {
    const ptr0 = passStringToWasm0(api_server, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(preferred) ? 0 : passStringToWasm0(preferred, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    const ret = wasm.get_gateway(ptr0, len0, ptr1, len1);
    return takeObject(ret);
}

/**
* @param {string} nym_api_url
* @returns {Promise<any>}
*/
export function current_network_topology(nym_api_url) {
    const ptr0 = passStringToWasm0(nym_api_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.current_network_topology(ptr0, len0);
    return takeObject(ret);
}

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}
/**
* Encode a payload
* @param {string} mime_type
* @param {Uint8Array} payload
* @returns {Uint8Array}
*/
export function encode_payload(mime_type, payload) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(mime_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.encode_payload(retptr, ptr0, len0, ptr1, len1);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        var r3 = getInt32Memory0()[retptr / 4 + 3];
        if (r3) {
            throw takeObject(r2);
        }
        var v2 = getArrayU8FromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 1);
        return v2;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* Create a new binary message with a user-specified `kind`, and `headers` as a string.
* @param {string} mime_type
* @param {Uint8Array} payload
* @param {string | undefined} headers
* @returns {Uint8Array}
*/
export function encode_payload_with_headers(mime_type, payload, headers) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(mime_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(headers) ? 0 : passStringToWasm0(headers, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        wasm.encode_payload_with_headers(retptr, ptr0, len0, ptr1, len1, ptr2, len2);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        var r3 = getInt32Memory0()[retptr / 4 + 3];
        if (r3) {
            throw takeObject(r2);
        }
        var v3 = getArrayU8FromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 1);
        return v3;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* Parse the `kind` and byte array `payload` from a byte array
* @param {Uint8Array} message
* @returns {EncodedPayload}
*/
export function decode_payload(message) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passArray8ToWasm0(message, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.decode_payload(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* Try parse a UTF-8 string from an array of bytes
* @param {Uint8Array} payload
* @returns {string}
*/
export function parse_utf8_string(payload) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.parse_utf8_string(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_free(r0, r1);
    }
}

/**
* Converts a UTF-8 string into an array of bytes
*
* This method is provided as a replacement for the mess of `atob`
* (https://developer.mozilla.org/en-US/docs/Web/API/atob) helpers provided by browsers and NodeJS.
*
* Feel free to use `atob` if you know you won't have problems with polyfills or encoding issues.
* @param {string} message
* @returns {Uint8Array}
*/
export function utf8_string_to_byte_array(message) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.utf8_string_to_byte_array(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var v1 = getArrayU8FromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 1);
        return v1;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {string} recipient
*/
export function validate_recipient(recipient) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(recipient, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.validate_recipient(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
*/
export function set_panic_hook() {
    wasm.set_panic_hook();
}

/**
* @returns {DebugWasm}
*/
export function default_debug() {
    const ret = wasm.default_debug();
    return DebugWasm.__wrap(ret);
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
function __wbg_adapter_442(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__ha2b01935184b2143(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

/**
*/
export class AcknowledgementsWasm {

    static __wrap(ptr) {
        const obj = Object.create(AcknowledgementsWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_acknowledgementswasm_free(ptr);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * sent acknowledgement is going to be delayed at any given mix node.
    * So for an ack going through three mix nodes, on average, it will take three times this value
    * until the packet reaches its destination.
    * @returns {bigint}
    */
    get average_ack_delay_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * sent acknowledgement is going to be delayed at any given mix node.
    * So for an ack going through three mix nodes, on average, it will take three times this value
    * until the packet reaches its destination.
    * @param {bigint} arg0
    */
    set average_ack_delay_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
    /**
    * Value multiplied with the expected round trip time of an acknowledgement packet before
    * it is assumed it was lost and retransmission of the data packet happens.
    * In an ideal network with 0 latency, this value would have been 1.
    * @returns {number}
    */
    get ack_wait_multiplier() {
        const ret = wasm.__wbg_get_acknowledgementswasm_ack_wait_multiplier(this.ptr);
        return ret;
    }
    /**
    * Value multiplied with the expected round trip time of an acknowledgement packet before
    * it is assumed it was lost and retransmission of the data packet happens.
    * In an ideal network with 0 latency, this value would have been 1.
    * @param {number} arg0
    */
    set ack_wait_multiplier(arg0) {
        wasm.__wbg_set_acknowledgementswasm_ack_wait_multiplier(this.ptr, arg0);
    }
    /**
    * Value added to the expected round trip time of an acknowledgement packet before
    * it is assumed it was lost and retransmission of the data packet happens.
    * In an ideal network with 0 latency, this value would have been 0.
    * @returns {bigint}
    */
    get ack_wait_addition_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_ack_wait_addition_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Value added to the expected round trip time of an acknowledgement packet before
    * it is assumed it was lost and retransmission of the data packet happens.
    * In an ideal network with 0 latency, this value would have been 0.
    * @param {bigint} arg0
    */
    set ack_wait_addition_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_ack_wait_addition_ms(this.ptr, arg0);
    }
}
/**
*/
export class AnonymousSenderTag {

    static __wrap(ptr) {
        const obj = Object.create(AnonymousSenderTag.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_anonymoussendertag_free(ptr);
    }
}
/**
*/
export class ClientStorage {

    static __wrap(ptr) {
        const obj = Object.create(ClientStorage.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_clientstorage_free(ptr);
    }
    /**
    * @param {string} client_id
    * @param {string} passphrase
    */
    constructor(client_id, passphrase) {
        const ptr0 = passStringToWasm0(client_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(passphrase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.clientstorage_new(ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * @param {string} client_id
    * @returns {Promise<any>}
    */
    static new_unencrypted(client_id) {
        const ptr0 = passStringToWasm0(client_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.clientstorage_new_unencrypted(ptr0, len0);
        return takeObject(ret);
    }
}
/**
*/
export class Config {

    static __wrap(ptr) {
        const obj = Object.create(Config.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_config_free(ptr);
    }
    /**
    * @param {string} id
    * @param {string} validator_server
    * @param {DebugWasm | undefined} debug
    */
    constructor(id, validator_server, debug) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(validator_server, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        let ptr2 = 0;
        if (!isLikeNone(debug)) {
            _assertClass(debug, DebugWasm);
            ptr2 = debug.ptr;
            debug.ptr = 0;
        }
        const ret = wasm.config_new(ptr0, len0, ptr1, len1, ptr2);
        return Config.__wrap(ret);
    }
}
/**
*/
export class CoverTrafficWasm {

    static __wrap(ptr) {
        const obj = Object.create(CoverTrafficWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_covertrafficwasm_free(ptr);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * it is going to take for another loop cover traffic message to be sent.
    * @returns {bigint}
    */
    get loop_cover_traffic_average_delay_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * it is going to take for another loop cover traffic message to be sent.
    * @param {bigint} arg0
    */
    set loop_cover_traffic_average_delay_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
    /**
    * Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    * Only applicable if `secondary_packet_size` is enabled.
    * @returns {number}
    */
    get cover_traffic_primary_size_ratio() {
        const ret = wasm.__wbg_get_acknowledgementswasm_ack_wait_multiplier(this.ptr);
        return ret;
    }
    /**
    * Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    * Only applicable if `secondary_packet_size` is enabled.
    * @param {number} arg0
    */
    set cover_traffic_primary_size_ratio(arg0) {
        wasm.__wbg_set_acknowledgementswasm_ack_wait_multiplier(this.ptr, arg0);
    }
    /**
    * Controls whether the dedicated loop cover traffic stream should be enabled.
    * (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    * @returns {boolean}
    */
    get disable_loop_cover_traffic_stream() {
        const ret = wasm.__wbg_get_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr);
        return ret !== 0;
    }
    /**
    * Controls whether the dedicated loop cover traffic stream should be enabled.
    * (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    * @param {boolean} arg0
    */
    set disable_loop_cover_traffic_stream(arg0) {
        wasm.__wbg_set_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr, arg0);
    }
}
/**
*/
export class DebugWasm {

    static __wrap(ptr) {
        const obj = Object.create(DebugWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_debugwasm_free(ptr);
    }
    /**
    * Defines all configuration options related to traffic streams.
    * @returns {TrafficWasm}
    */
    get traffic() {
        const ret = wasm.__wbg_get_debugwasm_traffic(this.ptr);
        return TrafficWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related to traffic streams.
    * @param {TrafficWasm} arg0
    */
    set traffic(arg0) {
        _assertClass(arg0, TrafficWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_traffic(this.ptr, ptr0);
    }
    /**
    * Defines all configuration options related to cover traffic stream(s).
    * @returns {CoverTrafficWasm}
    */
    get cover_traffic() {
        const ret = wasm.__wbg_get_debugwasm_cover_traffic(this.ptr);
        return CoverTrafficWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related to cover traffic stream(s).
    * @param {CoverTrafficWasm} arg0
    */
    set cover_traffic(arg0) {
        _assertClass(arg0, CoverTrafficWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_cover_traffic(this.ptr, ptr0);
    }
    /**
    * Defines all configuration options related to the gateway connection.
    * @returns {GatewayConnectionWasm}
    */
    get gateway_connection() {
        const ret = wasm.__wbg_get_debugwasm_gateway_connection(this.ptr);
        return GatewayConnectionWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related to the gateway connection.
    * @param {GatewayConnectionWasm} arg0
    */
    set gateway_connection(arg0) {
        _assertClass(arg0, GatewayConnectionWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_gateway_connection(this.ptr, ptr0);
    }
    /**
    * Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    * @returns {AcknowledgementsWasm}
    */
    get acknowledgements() {
        const ret = wasm.__wbg_get_debugwasm_acknowledgements(this.ptr);
        return AcknowledgementsWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    * @param {AcknowledgementsWasm} arg0
    */
    set acknowledgements(arg0) {
        _assertClass(arg0, AcknowledgementsWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_acknowledgements(this.ptr, ptr0);
    }
    /**
    * Defines all configuration options related topology, such as refresh rates or timeouts.
    * @returns {TopologyWasm}
    */
    get topology() {
        const ret = wasm.__wbg_get_debugwasm_topology(this.ptr);
        return TopologyWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related topology, such as refresh rates or timeouts.
    * @param {TopologyWasm} arg0
    */
    set topology(arg0) {
        _assertClass(arg0, TopologyWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_topology(this.ptr, ptr0);
    }
    /**
    * Defines all configuration options related to reply SURBs.
    * @returns {ReplySurbsWasm}
    */
    get reply_surbs() {
        const ret = wasm.__wbg_get_debugwasm_reply_surbs(this.ptr);
        return ReplySurbsWasm.__wrap(ret);
    }
    /**
    * Defines all configuration options related to reply SURBs.
    * @param {ReplySurbsWasm} arg0
    */
    set reply_surbs(arg0) {
        _assertClass(arg0, ReplySurbsWasm);
        var ptr0 = arg0.ptr;
        arg0.ptr = 0;
        wasm.__wbg_set_debugwasm_reply_surbs(this.ptr, ptr0);
    }
}
/**
*/
export class GatewayConnectionWasm {

    static __wrap(ptr) {
        const obj = Object.create(GatewayConnectionWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gatewayconnectionwasm_free(ptr);
    }
    /**
    * How long we're willing to wait for a response to a message sent to the gateway,
    * before giving up on it.
    * @returns {bigint}
    */
    get gateway_response_timeout_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * How long we're willing to wait for a response to a message sent to the gateway,
    * before giving up on it.
    * @param {bigint} arg0
    */
    set gateway_response_timeout_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
}
/**
*/
export class GatewayEndpointConfig {

    static __wrap(ptr) {
        const obj = Object.create(GatewayEndpointConfig.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_gatewayendpointconfig_free(ptr);
    }
    /**
    * gateway_id specifies ID of the gateway to which the client should send messages.
    * If initially omitted, a random gateway will be chosen from the available topology.
    * @returns {string}
    */
    get gateway_id() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_gatewayendpointconfig_gateway_id(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * gateway_id specifies ID of the gateway to which the client should send messages.
    * If initially omitted, a random gateway will be chosen from the available topology.
    * @param {string} arg0
    */
    set gateway_id(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_gatewayendpointconfig_gateway_id(this.ptr, ptr0, len0);
    }
    /**
    * Address of the gateway owner to which the client should send messages.
    * @returns {string}
    */
    get gateway_owner() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_gatewayendpointconfig_gateway_owner(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Address of the gateway owner to which the client should send messages.
    * @param {string} arg0
    */
    set gateway_owner(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_gatewayendpointconfig_gateway_owner(this.ptr, ptr0, len0);
    }
    /**
    * Address of the gateway listener to which all client requests should be sent.
    * @returns {string}
    */
    get gateway_listener() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_gatewayendpointconfig_gateway_listener(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * Address of the gateway listener to which all client requests should be sent.
    * @param {string} arg0
    */
    set gateway_listener(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_gatewayendpointconfig_gateway_listener(this.ptr, ptr0, len0);
    }
    /**
    * @param {string} gateway_id
    * @param {string} gateway_owner
    * @param {string} gateway_listener
    */
    constructor(gateway_id, gateway_owner, gateway_listener) {
        const ptr0 = passStringToWasm0(gateway_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(gateway_owner, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(gateway_listener, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.gatewayendpointconfig_new(ptr0, len0, ptr1, len1, ptr2, len2);
        return GatewayEndpointConfig.__wrap(ret);
    }
}
/**
*/
export class NodeTestResult {

    static __wrap(ptr) {
        const obj = Object.create(NodeTestResult.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nodetestresult_free(ptr);
    }
    /**
    * @returns {number}
    */
    get sent_packets() {
        const ret = wasm.__wbg_get_nodetestresult_sent_packets(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set sent_packets(arg0) {
        wasm.__wbg_set_nodetestresult_sent_packets(this.ptr, arg0);
    }
    /**
    * @returns {number}
    */
    get received_packets() {
        const ret = wasm.__wbg_get_nodetestresult_received_packets(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set received_packets(arg0) {
        wasm.__wbg_set_nodetestresult_received_packets(this.ptr, arg0);
    }
    /**
    * @returns {number}
    */
    get received_acks() {
        const ret = wasm.__wbg_get_nodetestresult_received_acks(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set received_acks(arg0) {
        wasm.__wbg_set_nodetestresult_received_acks(this.ptr, arg0);
    }
    /**
    * @returns {number}
    */
    get duplicate_packets() {
        const ret = wasm.__wbg_get_nodetestresult_duplicate_packets(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set duplicate_packets(arg0) {
        wasm.__wbg_set_nodetestresult_duplicate_packets(this.ptr, arg0);
    }
    /**
    * @returns {number}
    */
    get duplicate_acks() {
        const ret = wasm.__wbg_get_nodetestresult_duplicate_acks(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set duplicate_acks(arg0) {
        wasm.__wbg_set_nodetestresult_duplicate_acks(this.ptr, arg0);
    }
    /**
    */
    log_details() {
        wasm.nodetestresult_log_details(this.ptr);
    }
    /**
    * @returns {number}
    */
    score() {
        const ret = wasm.nodetestresult_score(this.ptr);
        return ret;
    }
}
/**
*/
export class NymClient {

    static __wrap(ptr) {
        const obj = Object.create(NymClient.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nymclient_free(ptr);
    }
    /**
    * @param {Config} config
    * @param {Function} on_message
    * @param {string | undefined} preferred_gateway
    * @param {string | undefined} storage_passphrase
    */
    constructor(config, on_message, preferred_gateway, storage_passphrase) {
        _assertClass(config, Config);
        var ptr0 = config.ptr;
        config.ptr = 0;
        var ptr1 = isLikeNone(preferred_gateway) ? 0 : passStringToWasm0(preferred_gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(storage_passphrase) ? 0 : passStringToWasm0(storage_passphrase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymclient_new(ptr0, addHeapObject(on_message), ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    self_address() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.nymclient_self_address(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} mixnode_identity
    * @param {number | undefined} num_test_packets
    * @returns {Promise<any>}
    */
    try_construct_test_packet_request(mixnode_identity, num_test_packets) {
        const ptr0 = passStringToWasm0(mixnode_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.nymclient_try_construct_test_packet_request(this.ptr, ptr0, len0, !isLikeNone(num_test_packets), isLikeNone(num_test_packets) ? 0 : num_test_packets);
        return takeObject(ret);
    }
    /**
    * @param {WasmNymTopology} topology
    * @returns {Promise<any>}
    */
    change_hardcoded_topology(topology) {
        _assertClass(topology, WasmNymTopology);
        var ptr0 = topology.ptr;
        topology.ptr = 0;
        const ret = wasm.nymclient_change_hardcoded_topology(this.ptr, ptr0);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    current_network_topology() {
        const ret = wasm.nymclient_current_network_topology(this.ptr);
        return takeObject(ret);
    }
    /**
    * Sends a test packet through the current network topology.
    * It's the responsibility of the caller to ensure the correct topology has been injected and
    * correct onmessage handlers have been setup.
    * @param {NymClientTestRequest} request
    * @returns {Promise<any>}
    */
    try_send_test_packets(request) {
        _assertClass(request, NymClientTestRequest);
        var ptr0 = request.ptr;
        request.ptr = 0;
        const ret = wasm.nymclient_try_send_test_packets(this.ptr, ptr0);
        return takeObject(ret);
    }
    /**
    * The simplest message variant where no additional information is attached.
    * You're simply sending your `data` to specified `recipient` without any tagging.
    *
    * Ends up with `NymMessage::Plain` variant
    * @param {Uint8Array} message
    * @param {string} recipient
    * @returns {Promise<any>}
    */
    send_regular_message(message, recipient) {
        const ptr0 = passArray8ToWasm0(message, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(recipient, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.nymclient_send_regular_message(this.ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
    /**
    * Creates a message used for a duplex anonymous communication where the recipient
    * will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
    *
    * Note that if reply_surbs is set to zero then
    * this variant requires the client having sent some reply_surbs in the past
    * (and thus the recipient also knowing our sender tag).
    *
    * Ends up with `NymMessage::Repliable` variant
    * @param {Uint8Array} message
    * @param {string} recipient
    * @param {number} reply_surbs
    * @returns {Promise<any>}
    */
    send_anonymous_message(message, recipient, reply_surbs) {
        const ptr0 = passArray8ToWasm0(message, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(recipient, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.nymclient_send_anonymous_message(this.ptr, ptr0, len0, ptr1, len1, reply_surbs);
        return takeObject(ret);
    }
    /**
    * Attempt to use our internally received and stored `ReplySurb` to send the message back
    * to specified recipient whilst not knowing its full identity (or even gateway).
    *
    * Ends up with `NymMessage::Reply` variant
    * @param {Uint8Array} message
    * @param {string} recipient_tag
    * @returns {Promise<any>}
    */
    send_reply(message, recipient_tag) {
        const ptr0 = passArray8ToWasm0(message, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(recipient_tag, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.nymclient_send_reply(this.ptr, ptr0, len0, ptr1, len1);
        return takeObject(ret);
    }
}
/**
*/
export class NymClientBuilder {

    static __wrap(ptr) {
        const obj = Object.create(NymClientBuilder.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nymclientbuilder_free(ptr);
    }
    /**
    * @param {Config} config
    * @param {Function} on_message
    * @param {string | undefined} preferred_gateway
    * @param {string | undefined} storage_passphrase
    */
    constructor(config, on_message, preferred_gateway, storage_passphrase) {
        _assertClass(config, Config);
        var ptr0 = config.ptr;
        config.ptr = 0;
        var ptr1 = isLikeNone(preferred_gateway) ? 0 : passStringToWasm0(preferred_gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(storage_passphrase) ? 0 : passStringToWasm0(storage_passphrase, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymclientbuilder_new(ptr0, addHeapObject(on_message), ptr1, len1, ptr2, len2);
        return NymClientBuilder.__wrap(ret);
    }
    /**
    * @param {WasmNymTopology} topology
    * @param {Function} on_message
    * @param {string | undefined} gateway
    * @returns {NymClientBuilder}
    */
    static new_tester(topology, on_message, gateway) {
        _assertClass(topology, WasmNymTopology);
        var ptr0 = topology.ptr;
        topology.ptr = 0;
        var ptr1 = isLikeNone(gateway) ? 0 : passStringToWasm0(gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.nymclientbuilder_new_tester(ptr0, addHeapObject(on_message), ptr1, len1);
        return NymClientBuilder.__wrap(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    start_client() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.nymclientbuilder_start_client(ptr);
        return takeObject(ret);
    }
}
/**
*/
export class NymClientTestRequest {

    static __wrap(ptr) {
        const obj = Object.create(NymClientTestRequest.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nymclienttestrequest_free(ptr);
    }
    /**
    * @returns {WasmNymTopology}
    */
    injectable_topology() {
        const ret = wasm.nymclienttestrequest_injectable_topology(this.ptr);
        return WasmNymTopology.__wrap(ret);
    }
}
/**
*/
export class NymNodeTester {

    static __wrap(ptr) {
        const obj = Object.create(NymNodeTester.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nymnodetester_free(ptr);
    }
    /**
    * @param {WasmNymTopology} topology
    * @param {string | undefined} id
    * @param {string | undefined} gateway
    */
    constructor(topology, id, gateway) {
        _assertClass(topology, WasmNymTopology);
        var ptr0 = topology.ptr;
        topology.ptr = 0;
        var ptr1 = isLikeNone(id) ? 0 : passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(gateway) ? 0 : passStringToWasm0(gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymnodetester_new(ptr0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * @param {string} api_url
    * @param {string | undefined} id
    * @param {string | undefined} gateway
    * @returns {Promise<any>}
    */
    static new_with_api(api_url, id, gateway) {
        const ptr0 = passStringToWasm0(api_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(id) ? 0 : passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(gateway) ? 0 : passStringToWasm0(gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymnodetester_new_with_api(ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    disconnect_from_gateway() {
        const ret = wasm.nymnodetester_disconnect_from_gateway(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    reconnect_to_gateway() {
        const ret = wasm.nymnodetester_reconnect_to_gateway(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} mixnode_identity
    * @param {bigint | undefined} timeout_millis
    * @param {number | undefined} num_test_packets
    * @returns {Promise<any>}
    */
    test_node(mixnode_identity, timeout_millis, num_test_packets) {
        const ptr0 = passStringToWasm0(mixnode_identity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.nymnodetester_test_node(this.ptr, ptr0, len0, !isLikeNone(timeout_millis), isLikeNone(timeout_millis) ? 0n : timeout_millis, !isLikeNone(num_test_packets), isLikeNone(num_test_packets) ? 0 : num_test_packets);
        return takeObject(ret);
    }
}
/**
*/
export class NymNodeTesterBuilder {

    static __wrap(ptr) {
        const obj = Object.create(NymNodeTesterBuilder.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nymnodetesterbuilder_free(ptr);
    }
    /**
    * @param {WasmNymTopology} base_topology
    * @param {string | undefined} id
    * @param {string | undefined} gateway
    */
    constructor(base_topology, id, gateway) {
        _assertClass(base_topology, WasmNymTopology);
        var ptr0 = base_topology.ptr;
        base_topology.ptr = 0;
        var ptr1 = isLikeNone(id) ? 0 : passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(gateway) ? 0 : passStringToWasm0(gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymnodetesterbuilder_new(ptr0, ptr1, len1, ptr2, len2);
        return NymNodeTesterBuilder.__wrap(ret);
    }
    /**
    * @param {string} api_url
    * @param {string | undefined} id
    * @param {string | undefined} gateway
    * @returns {Promise<any>}
    */
    static new_with_api(api_url, id, gateway) {
        const ptr0 = passStringToWasm0(api_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(id) ? 0 : passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(gateway) ? 0 : passStringToWasm0(gateway, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.nymnodetesterbuilder_new_with_api(ptr0, len0, ptr1, len1, ptr2, len2);
        return takeObject(ret);
    }
    /**
    * @returns {Promise<any>}
    */
    setup_client() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.nymnodetesterbuilder_setup_client(ptr);
        return takeObject(ret);
    }
}
/**
*/
export class ReplySurbsWasm {

    static __wrap(ptr) {
        const obj = Object.create(ReplySurbsWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_replysurbswasm_free(ptr);
    }
    /**
    * Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    * It can only allow to go below that value if its to request additional reply surbs.
    * @returns {number}
    */
    get minimum_reply_surb_storage_threshold() {
        const ret = wasm.__wbg_get_replysurbswasm_minimum_reply_surb_storage_threshold(this.ptr);
        return ret >>> 0;
    }
    /**
    * Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    * It can only allow to go below that value if its to request additional reply surbs.
    * @param {number} arg0
    */
    set minimum_reply_surb_storage_threshold(arg0) {
        wasm.__wbg_set_replysurbswasm_minimum_reply_surb_storage_threshold(this.ptr, arg0);
    }
    /**
    * Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    * @returns {number}
    */
    get maximum_reply_surb_storage_threshold() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_surb_storage_threshold(this.ptr);
        return ret >>> 0;
    }
    /**
    * Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    * @param {number} arg0
    */
    set maximum_reply_surb_storage_threshold(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_surb_storage_threshold(this.ptr, arg0);
    }
    /**
    * Defines the minimum number of reply surbs the client would request.
    * @returns {number}
    */
    get minimum_reply_surb_request_size() {
        const ret = wasm.__wbg_get_replysurbswasm_minimum_reply_surb_request_size(this.ptr);
        return ret >>> 0;
    }
    /**
    * Defines the minimum number of reply surbs the client would request.
    * @param {number} arg0
    */
    set minimum_reply_surb_request_size(arg0) {
        wasm.__wbg_set_replysurbswasm_minimum_reply_surb_request_size(this.ptr, arg0);
    }
    /**
    * Defines the maximum number of reply surbs the client would request.
    * @returns {number}
    */
    get maximum_reply_surb_request_size() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_surb_request_size(this.ptr);
        return ret >>> 0;
    }
    /**
    * Defines the maximum number of reply surbs the client would request.
    * @param {number} arg0
    */
    set maximum_reply_surb_request_size(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_surb_request_size(this.ptr, arg0);
    }
    /**
    * Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    * @returns {number}
    */
    get maximum_allowed_reply_surb_request_size() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_allowed_reply_surb_request_size(this.ptr);
        return ret >>> 0;
    }
    /**
    * Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    * @param {number} arg0
    */
    set maximum_allowed_reply_surb_request_size(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_allowed_reply_surb_request_size(this.ptr, arg0);
    }
    /**
    * Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    * for more even though in theory they wouldn't need to.
    * @returns {bigint}
    */
    get maximum_reply_surb_rerequest_waiting_period_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    * for more even though in theory they wouldn't need to.
    * @param {bigint} arg0
    */
    set maximum_reply_surb_rerequest_waiting_period_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
    /**
    * Defines maximum amount of time the client is going to wait for reply surbs before
    * deciding it's never going to get them and would drop all pending messages
    * @returns {bigint}
    */
    get maximum_reply_surb_drop_waiting_period_ms() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Defines maximum amount of time the client is going to wait for reply surbs before
    * deciding it's never going to get them and would drop all pending messages
    * @param {bigint} arg0
    */
    set maximum_reply_surb_drop_waiting_period_ms(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr, arg0);
    }
    /**
    * Defines maximum amount of time given reply surb is going to be valid for.
    * This is going to be superseded by key rotation once implemented.
    * @returns {bigint}
    */
    get maximum_reply_surb_age_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_ack_wait_addition_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Defines maximum amount of time given reply surb is going to be valid for.
    * This is going to be superseded by key rotation once implemented.
    * @param {bigint} arg0
    */
    set maximum_reply_surb_age_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_ack_wait_addition_ms(this.ptr, arg0);
    }
    /**
    * Defines maximum amount of time given reply key is going to be valid for.
    * This is going to be superseded by key rotation once implemented.
    * @returns {bigint}
    */
    get maximum_reply_key_age_ms() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_key_age_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * Defines maximum amount of time given reply key is going to be valid for.
    * This is going to be superseded by key rotation once implemented.
    * @param {bigint} arg0
    */
    set maximum_reply_key_age_ms(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_key_age_ms(this.ptr, arg0);
    }
}
/**
*/
export class TopologyWasm {

    static __wrap(ptr) {
        const obj = Object.create(TopologyWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_topologywasm_free(ptr);
    }
    /**
    * The uniform delay every which clients are querying the directory server
    * to try to obtain a compatible network topology to send sphinx packets through.
    * @returns {bigint}
    */
    get topology_refresh_rate_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The uniform delay every which clients are querying the directory server
    * to try to obtain a compatible network topology to send sphinx packets through.
    * @param {bigint} arg0
    */
    set topology_refresh_rate_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
    /**
    * During topology refresh, test packets are sent through every single possible network
    * path. This timeout determines waiting period until it is decided that the packet
    * did not reach its destination.
    * @returns {bigint}
    */
    get topology_resolution_timeout_ms() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * During topology refresh, test packets are sent through every single possible network
    * path. This timeout determines waiting period until it is decided that the packet
    * did not reach its destination.
    * @param {bigint} arg0
    */
    set topology_resolution_timeout_ms(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr, arg0);
    }
    /**
    * Specifies whether the client should not refresh the network topology after obtaining
    * the first valid instance.
    * Supersedes `topology_refresh_rate_ms`.
    * @returns {boolean}
    */
    get disable_refreshing() {
        const ret = wasm.__wbg_get_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr);
        return ret !== 0;
    }
    /**
    * Specifies whether the client should not refresh the network topology after obtaining
    * the first valid instance.
    * Supersedes `topology_refresh_rate_ms`.
    * @param {boolean} arg0
    */
    set disable_refreshing(arg0) {
        wasm.__wbg_set_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr, arg0);
    }
}
/**
*/
export class TrafficWasm {

    static __wrap(ptr) {
        const obj = Object.create(TrafficWasm.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_trafficwasm_free(ptr);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * sent packet is going to be delayed at any given mix node.
    * So for a packet going through three mix nodes, on average, it will take three times this value
    * until the packet reaches its destination.
    * @returns {bigint}
    */
    get average_packet_delay_ms() {
        const ret = wasm.__wbg_get_acknowledgementswasm_average_ack_delay_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * sent packet is going to be delayed at any given mix node.
    * So for a packet going through three mix nodes, on average, it will take three times this value
    * until the packet reaches its destination.
    * @param {bigint} arg0
    */
    set average_packet_delay_ms(arg0) {
        wasm.__wbg_set_acknowledgementswasm_average_ack_delay_ms(this.ptr, arg0);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * it is going to take another 'real traffic stream' message to be sent.
    * If no real packets are available and cover traffic is enabled,
    * a loop cover message is sent instead in order to preserve the rate.
    * @returns {bigint}
    */
    get message_sending_average_delay_ms() {
        const ret = wasm.__wbg_get_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * The parameter of Poisson distribution determining how long, on average,
    * it is going to take another 'real traffic stream' message to be sent.
    * If no real packets are available and cover traffic is enabled,
    * a loop cover message is sent instead in order to preserve the rate.
    * @param {bigint} arg0
    */
    set message_sending_average_delay_ms(arg0) {
        wasm.__wbg_set_replysurbswasm_maximum_reply_surb_drop_waiting_period_ms(this.ptr, arg0);
    }
    /**
    * Controls whether the main packet stream constantly produces packets according to the predefined
    * poisson distribution.
    * @returns {boolean}
    */
    get disable_main_poisson_packet_distribution() {
        const ret = wasm.__wbg_get_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr);
        return ret !== 0;
    }
    /**
    * Controls whether the main packet stream constantly produces packets according to the predefined
    * poisson distribution.
    * @param {boolean} arg0
    */
    set disable_main_poisson_packet_distribution(arg0) {
        wasm.__wbg_set_covertrafficwasm_disable_loop_cover_traffic_stream(this.ptr, arg0);
    }
    /**
    * Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
    * @returns {boolean}
    */
    get use_extended_packet_size() {
        const ret = wasm.__wbg_get_trafficwasm_use_extended_packet_size(this.ptr);
        return ret !== 0;
    }
    /**
    * Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
    * @param {boolean} arg0
    */
    set use_extended_packet_size(arg0) {
        wasm.__wbg_set_trafficwasm_use_extended_packet_size(this.ptr, arg0);
    }
    /**
    * Controls whether the sent packets should use outfox as opposed to the default sphinx.
    * @returns {boolean}
    */
    get use_outfox() {
        const ret = wasm.__wbg_get_trafficwasm_use_outfox(this.ptr);
        return ret !== 0;
    }
    /**
    * Controls whether the sent packets should use outfox as opposed to the default sphinx.
    * @param {boolean} arg0
    */
    set use_outfox(arg0) {
        wasm.__wbg_set_trafficwasm_use_outfox(this.ptr, arg0);
    }
}
/**
*/
export class WasmGateway {

    static __wrap(ptr) {
        const obj = Object.create(WasmGateway.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmgateway_free(ptr);
    }
    /**
    * @returns {string}
    */
    get owner() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmgateway_owner(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set owner(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmgateway_owner(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get host() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmgateway_host(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set host(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmgateway_host(this.ptr, ptr0, len0);
    }
    /**
    * @returns {number}
    */
    get mix_port() {
        const ret = wasm.__wbg_get_wasmgateway_mix_port(this.ptr);
        return ret;
    }
    /**
    * @param {number} arg0
    */
    set mix_port(arg0) {
        wasm.__wbg_set_wasmgateway_mix_port(this.ptr, arg0);
    }
    /**
    * @returns {number}
    */
    get clients_port() {
        const ret = wasm.__wbg_get_wasmgateway_clients_port(this.ptr);
        return ret;
    }
    /**
    * @param {number} arg0
    */
    set clients_port(arg0) {
        wasm.__wbg_set_wasmgateway_clients_port(this.ptr, arg0);
    }
    /**
    * @returns {string}
    */
    get identity_key() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmgateway_identity_key(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set identity_key(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmgateway_identity_key(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get sphinx_key() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmgateway_sphinx_key(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set sphinx_key(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmgateway_sphinx_key(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get version() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmgateway_version(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set version(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmgateway_version(this.ptr, ptr0, len0);
    }
    /**
    * @param {string} owner
    * @param {string} host
    * @param {number} mix_port
    * @param {number} clients_port
    * @param {string} identity_key
    * @param {string} sphinx_key
    * @param {string} version
    */
    constructor(owner, host, mix_port, clients_port, identity_key, sphinx_key, version) {
        const ptr0 = passStringToWasm0(owner, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(host, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(identity_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(sphinx_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(version, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.wasmgateway_new(ptr0, len0, ptr1, len1, mix_port, clients_port, ptr2, len2, ptr3, len3, ptr4, len4);
        return WasmGateway.__wrap(ret);
    }
}
/**
*/
export class WasmMixNode {

    static __wrap(ptr) {
        const obj = Object.create(WasmMixNode.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmmixnode_free(ptr);
    }
    /**
    * @returns {number}
    */
    get mix_id() {
        const ret = wasm.__wbg_get_wasmmixnode_mix_id(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} arg0
    */
    set mix_id(arg0) {
        wasm.__wbg_set_wasmmixnode_mix_id(this.ptr, arg0);
    }
    /**
    * @returns {string}
    */
    get owner() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmmixnode_owner(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set owner(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmmixnode_owner(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get host() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmmixnode_host(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set host(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmmixnode_host(this.ptr, ptr0, len0);
    }
    /**
    * @returns {number}
    */
    get mix_port() {
        const ret = wasm.__wbg_get_wasmmixnode_mix_port(this.ptr);
        return ret;
    }
    /**
    * @param {number} arg0
    */
    set mix_port(arg0) {
        wasm.__wbg_set_wasmmixnode_mix_port(this.ptr, arg0);
    }
    /**
    * @returns {string}
    */
    get identity_key() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmmixnode_identity_key(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set identity_key(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmmixnode_identity_key(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get sphinx_key() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmmixnode_sphinx_key(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set sphinx_key(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmmixnode_sphinx_key(this.ptr, ptr0, len0);
    }
    /**
    * @returns {number}
    */
    get layer() {
        const ret = wasm.__wbg_get_wasmmixnode_layer(this.ptr);
        return ret;
    }
    /**
    * @param {number} arg0
    */
    set layer(arg0) {
        wasm.__wbg_set_wasmmixnode_layer(this.ptr, arg0);
    }
    /**
    * @returns {string}
    */
    get version() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.__wbg_get_wasmmixnode_version(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} arg0
    */
    set version(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_wasmmixnode_version(this.ptr, ptr0, len0);
    }
    /**
    * @param {number} mix_id
    * @param {string} owner
    * @param {string} host
    * @param {number} mix_port
    * @param {string} identity_key
    * @param {string} sphinx_key
    * @param {number} layer
    * @param {string} version
    */
    constructor(mix_id, owner, host, mix_port, identity_key, sphinx_key, layer, version) {
        const ptr0 = passStringToWasm0(owner, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(host, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(identity_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passStringToWasm0(sphinx_key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len3 = WASM_VECTOR_LEN;
        const ptr4 = passStringToWasm0(version, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len4 = WASM_VECTOR_LEN;
        const ret = wasm.wasmmixnode_new(mix_id, ptr0, len0, ptr1, len1, mix_port, ptr2, len2, ptr3, len3, layer, ptr4, len4);
        return WasmMixNode.__wrap(ret);
    }
}
/**
*/
export class WasmNymTopology {

    static __wrap(ptr) {
        const obj = Object.create(WasmNymTopology.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmnymtopology_free(ptr);
    }
    /**
    * @param {any} mixnodes
    * @param {any} gateways
    */
    constructor(mixnodes, gateways) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.wasmnymtopology_new(retptr, addHeapObject(mixnodes), addHeapObject(gateways));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return WasmNymTopology.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    */
    print() {
        wasm.wasmnymtopology_print(this.ptr);
    }
}

export function __wbindgen_object_drop_ref(arg0) {
    takeObject(arg0);
};

export function __wbindgen_cb_drop(arg0) {
    const obj = takeObject(arg0).original;
    if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
    }
    const ret = false;
    return ret;
};

export function __wbindgen_string_get(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_nodetestresult_new(arg0) {
    const ret = NodeTestResult.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_nymnodetester_new(arg0) {
    const ret = NymNodeTester.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_nymclient_new(arg0) {
    const ret = NymClient.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_nymnodetesterbuilder_new(arg0) {
    const ret = NymNodeTesterBuilder.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_clientstorage_new(arg0) {
    const ret = ClientStorage.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_wasmnymtopology_new(arg0) {
    const ret = WasmNymTopology.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_nymclienttestrequest_new(arg0) {
    const ret = NymClientTestRequest.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbindgen_is_string(arg0) {
    const ret = typeof(getObject(arg0)) === 'string';
    return ret;
};

export function __wbindgen_is_bigint(arg0) {
    const ret = typeof(getObject(arg0)) === 'bigint';
    return ret;
};

export function __wbindgen_bigint_from_u64(arg0) {
    const ret = BigInt.asUintN(64, arg0);
    return addHeapObject(ret);
};

export function __wbindgen_jsval_eq(arg0, arg1) {
    const ret = getObject(arg0) === getObject(arg1);
    return ret;
};

export function __wbindgen_is_object(arg0) {
    const val = getObject(arg0);
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
};

export function __wbindgen_is_undefined(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
};

export function __wbindgen_in(arg0, arg1) {
    const ret = getObject(arg0) in getObject(arg1);
    return ret;
};

export function __wbindgen_number_new(arg0) {
    const ret = arg0;
    return addHeapObject(ret);
};

export function __wbindgen_string_new(arg0, arg1) {
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

export function __wbindgen_error_new(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_new_abda76e883ba8a5f() {
    const ret = new Error();
    return addHeapObject(ret);
};

export function __wbg_stack_658279fe44541cf6(arg0, arg1) {
    const ret = getObject(arg1).stack;
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_error_f851667af71bcfc6(arg0, arg1) {
    try {
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_free(arg0, arg1);
    }
};

export function __wbg_gatewayendpointconfig_new(arg0) {
    const ret = GatewayEndpointConfig.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_clearInterval_bd072ecb096d9775(arg0) {
    const ret = clearInterval(takeObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_setInterval_edede8e2124cbb00() { return handleError(function (arg0, arg1) {
    const ret = setInterval(getObject(arg0), arg1);
    return addHeapObject(ret);
}, arguments) };

export function __wbg_error_d8f8bcfc5d63b5bb(arg0, arg1) {
    console.error(getStringFromWasm0(arg0, arg1));
};

export function __wbg_log_56ad965dcd7a8d1e(arg0, arg1) {
    console.log(getStringFromWasm0(arg0, arg1));
};

export function __wbg_warn_6b6312ae47b4000a(arg0, arg1) {
    console.warn(getStringFromWasm0(arg0, arg1));
};

export function __wbindgen_object_clone_ref(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
};

export function __wbindgen_jsval_loose_eq(arg0, arg1) {
    const ret = getObject(arg0) == getObject(arg1);
    return ret;
};

export function __wbindgen_boolean_get(arg0) {
    const v = getObject(arg0);
    const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
    return ret;
};

export function __wbindgen_number_get(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'number' ? obj : undefined;
    getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbg_String_88810dfeb4021902(arg0, arg1) {
    const ret = String(getObject(arg1));
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_getwithrefkey_5e6d9547403deab8(arg0, arg1) {
    const ret = getObject(arg0)[getObject(arg1)];
    return addHeapObject(ret);
};

export function __wbg_set_841ac57cff3d672b(arg0, arg1, arg2) {
    getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
};

export function __wbg_Window_2323448e22bf340f(arg0) {
    const ret = getObject(arg0).Window;
    return addHeapObject(ret);
};

export function __wbg_WorkerGlobalScope_4f52a4f4757baa51(arg0) {
    const ret = getObject(arg0).WorkerGlobalScope;
    return addHeapObject(ret);
};

export function __wbg_global_bb13ba737d1fd37d(arg0) {
    const ret = getObject(arg0).global;
    return addHeapObject(ret);
};

export function __wbg_indexedDB_553c6eee256a5956() { return handleError(function (arg0) {
    const ret = getObject(arg0).indexedDB;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments) };

export function __wbg_setTimeout_5367316a4a2049a4(arg0, arg1) {
    const ret = setTimeout(getObject(arg0), arg1);
    return ret;
};

export function __wbg_static_accessor_performance_0c2e02df5be582ca() {
    const ret = performance;
    return addHeapObject(ret);
};

export function __wbg_anonymoussendertag_new(arg0) {
    const ret = AnonymousSenderTag.__wrap(arg0);
    return addHeapObject(ret);
};

export function __wbg_fetch_3a1be51760e1f8eb(arg0) {
    const ret = fetch(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_indexedDB_050f0962ab607ac5() { return handleError(function (arg0) {
    const ret = getObject(arg0).indexedDB;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments) };

export function __wbg_indexedDB_8d9e9ab4616df7f0() { return handleError(function (arg0) {
    const ret = getObject(arg0).indexedDB;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments) };

export function __wbg_fetch_749a56934f95c96c(arg0, arg1) {
    const ret = getObject(arg0).fetch(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_target_bf704b7db7ad1387(arg0) {
    const ret = getObject(arg0).target;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

export function __wbg_readyState_9c0f66433e329c9e(arg0) {
    const ret = getObject(arg0).readyState;
    return ret;
};

export function __wbg_setonopen_9ce48dce57e549b5(arg0, arg1) {
    getObject(arg0).onopen = getObject(arg1);
};

export function __wbg_setonerror_02393260b3e29972(arg0, arg1) {
    getObject(arg0).onerror = getObject(arg1);
};

export function __wbg_setonclose_4ce49fd8fd7783fb(arg0, arg1) {
    getObject(arg0).onclose = getObject(arg1);
};

export function __wbg_setonmessage_c5a806b62a0c5607(arg0, arg1) {
    getObject(arg0).onmessage = getObject(arg1);
};

export function __wbg_setbinaryType_ee55743ddf4beb37(arg0, arg1) {
    getObject(arg0).binaryType = takeObject(arg1);
};

export function __wbg_new_d29e507f6606de91() { return handleError(function (arg0, arg1) {
    const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_close_45d053bea59e7746() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments) };

export function __wbg_send_80b256d87a6779e5() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).send(getStringFromWasm0(arg1, arg2));
}, arguments) };

export function __wbg_send_640853f8eb0f0385() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).send(getArrayU8FromWasm0(arg1, arg2));
}, arguments) };

export function __wbg_instanceof_Response_eaa426220848a39e(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Response;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_url_74285ddf2747cb3d(arg0, arg1) {
    const ret = getObject(arg1).url;
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_status_c4ef3dd591e63435(arg0) {
    const ret = getObject(arg0).status;
    return ret;
};

export function __wbg_headers_fd64ad685cf22e5d(arg0) {
    const ret = getObject(arg0).headers;
    return addHeapObject(ret);
};

export function __wbg_arrayBuffer_4c27b6f00c530232() { return handleError(function (arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_text_1169d752cc697903() { return handleError(function (arg0) {
    const ret = getObject(arg0).text();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_setonversionchange_840d65cd0888dfb0(arg0, arg1) {
    getObject(arg0).onversionchange = getObject(arg1);
};

export function __wbg_createObjectStore_d3e2789c13dde1fc() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).createObjectStore(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_transaction_cce96cbebd81fe1c() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).transaction(getStringFromWasm0(arg1, arg2), takeObject(arg3));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_setonabort_404bee3b9940d03d(arg0, arg1) {
    getObject(arg0).onabort = getObject(arg1);
};

export function __wbg_setoncomplete_3e57a8cec8327f66(arg0, arg1) {
    getObject(arg0).oncomplete = getObject(arg1);
};

export function __wbg_setonerror_00051c0213f27b2c(arg0, arg1) {
    getObject(arg0).onerror = getObject(arg1);
};

export function __wbg_objectStore_f17976b0e6377830() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).objectStore(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_2d0053ee81e4dd2a() { return handleError(function () {
    const ret = new Headers();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_append_de37df908812970d() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).append(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments) };

export function __wbg_get_6285bf458a1ee758() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).get(getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_put_84e7fc93eee27b28() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).put(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_name_e2b7d53714dcd5c4(arg0, arg1) {
    const ret = getObject(arg1).name;
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_message_a7af3ee0cc0fe28d(arg0, arg1) {
    const ret = getObject(arg1).message;
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbg_code_b09504754e0520f1(arg0) {
    const ret = getObject(arg0).code;
    return ret;
};

export function __wbg_now_8172cd917e5eda6b(arg0) {
    const ret = getObject(arg0).now();
    return ret;
};

export function __wbg_newwithstrandinit_05d7180788420c40() { return handleError(function (arg0, arg1, arg2) {
    const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_setonblocked_e66d6be5c879980d(arg0, arg1) {
    getObject(arg0).onblocked = getObject(arg1);
};

export function __wbg_setonupgradeneeded_17d0b9530f1e0cac(arg0, arg1) {
    getObject(arg0).onupgradeneeded = getObject(arg1);
};

export function __wbg_oldVersion_988bd06e72c257b1(arg0) {
    const ret = getObject(arg0).oldVersion;
    return ret;
};

export function __wbg_data_7b1f01f4e6a64fbe(arg0) {
    const ret = getObject(arg0).data;
    return addHeapObject(ret);
};

export function __wbg_open_c5d5fb2df44b9d10() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2), arg3 >>> 0);
    return addHeapObject(ret);
}, arguments) };

export function __wbg_result_9e399c14676970d9() { return handleError(function (arg0) {
    const ret = getObject(arg0).result;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_error_aacf5ac191e54ed0() { return handleError(function (arg0) {
    const ret = getObject(arg0).error;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments) };

export function __wbg_readyState_fb287f170113917c(arg0) {
    const ret = getObject(arg0).readyState;
    return addHeapObject(ret);
};

export function __wbg_setonsuccess_5f71593bc51653a3(arg0, arg1) {
    getObject(arg0).onsuccess = getObject(arg1);
};

export function __wbg_setonerror_d5771cc5bf9ea74c(arg0, arg1) {
    getObject(arg0).onerror = getObject(arg1);
};

export function __wbg_instanceof_Blob_d18d26355bccfd22(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Blob;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_randomFillSync_85b3f4c52c56c313(arg0, arg1, arg2) {
    getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
};

export function __wbg_getRandomValues_cd175915511f705e(arg0, arg1) {
    getObject(arg0).getRandomValues(getObject(arg1));
};

export function __wbg_self_7eede1f4488bf346() { return handleError(function () {
    const ret = self.self;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_crypto_c909fb428dcbddb6(arg0) {
    const ret = getObject(arg0).crypto;
    return addHeapObject(ret);
};

export function __wbg_msCrypto_511eefefbfc70ae4(arg0) {
    const ret = getObject(arg0).msCrypto;
    return addHeapObject(ret);
};

export function __wbg_static_accessor_MODULE_ef3aa2eb251158a5() {
    const ret = module;
    return addHeapObject(ret);
};

export function __wbg_require_900d5c3984fe7703(arg0, arg1, arg2) {
    const ret = getObject(arg0).require(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
};

export function __wbg_getRandomValues_307049345d0bd88c(arg0) {
    const ret = getObject(arg0).getRandomValues;
    return addHeapObject(ret);
};

export function __wbg_getRandomValues_37fa2ca9e4e07fab() { return handleError(function (arg0, arg1) {
    getObject(arg0).getRandomValues(getObject(arg1));
}, arguments) };

export function __wbg_randomFillSync_dc1e9a60c158336d() { return handleError(function (arg0, arg1) {
    getObject(arg0).randomFillSync(takeObject(arg1));
}, arguments) };

export function __wbg_crypto_c48a774b022d20ac(arg0) {
    const ret = getObject(arg0).crypto;
    return addHeapObject(ret);
};

export function __wbg_process_298734cf255a885d(arg0) {
    const ret = getObject(arg0).process;
    return addHeapObject(ret);
};

export function __wbg_versions_e2e78e134e3e5d01(arg0) {
    const ret = getObject(arg0).versions;
    return addHeapObject(ret);
};

export function __wbg_node_1cd7a5d853dbea79(arg0) {
    const ret = getObject(arg0).node;
    return addHeapObject(ret);
};

export function __wbg_msCrypto_bcb970640f50a1e8(arg0) {
    const ret = getObject(arg0).msCrypto;
    return addHeapObject(ret);
};

export function __wbg_require_8f08ceecec0f4fee() { return handleError(function () {
    const ret = module.require;
    return addHeapObject(ret);
}, arguments) };

export function __wbindgen_is_function(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
};

export function __wbg_get_57245cc7d7c7619d(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};

export function __wbg_length_6e3bbe7c8bd4dbd8(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

export function __wbg_new_1d9a920c6bfc44a8() {
    const ret = new Array();
    return addHeapObject(ret);
};

export function __wbg_newnoargs_b5b063fc6c2f0376(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_next_579e583d33566a86(arg0) {
    const ret = getObject(arg0).next;
    return addHeapObject(ret);
};

export function __wbg_next_aaef7c8aa5e212ac() { return handleError(function (arg0) {
    const ret = getObject(arg0).next();
    return addHeapObject(ret);
}, arguments) };

export function __wbg_done_1b73b0672e15f234(arg0) {
    const ret = getObject(arg0).done;
    return ret;
};

export function __wbg_value_1ccc36bc03462d71(arg0) {
    const ret = getObject(arg0).value;
    return addHeapObject(ret);
};

export function __wbg_iterator_6f9d4f28845f426c() {
    const ret = Symbol.iterator;
    return addHeapObject(ret);
};

export function __wbg_get_765201544a2b6869() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_call_97ae9d8645dc388b() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_new_0b9bfdd97583284e() {
    const ret = new Object();
    return addHeapObject(ret);
};

export function __wbg_self_6d479506f72c6a71() { return handleError(function () {
    const ret = self.self;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_window_f2557cc78490aceb() { return handleError(function () {
    const ret = window.window;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_globalThis_7f206bda628d5286() { return handleError(function () {
    const ret = globalThis.globalThis;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_global_ba75c50d1cf384f4() { return handleError(function () {
    const ret = global.global;
    return addHeapObject(ret);
}, arguments) };

export function __wbg_set_a68214f35c417fa9(arg0, arg1, arg2) {
    getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
};

export function __wbg_from_7ce3cb27cb258569(arg0) {
    const ret = Array.from(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_isArray_27c46c67f498e15d(arg0) {
    const ret = Array.isArray(getObject(arg0));
    return ret;
};

export function __wbg_instanceof_ArrayBuffer_e5e48f4762c5610b(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ArrayBuffer;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_new_8d2af00bc1e329ee(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

export function __wbg_call_168da88779e35f61() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_call_3999bee59e9f7719() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
    return addHeapObject(ret);
}, arguments) };

export function __wbg_isSafeInteger_dfa0593e8d7ac35a(arg0) {
    const ret = Number.isSafeInteger(getObject(arg0));
    return ret;
};

export function __wbg_getTime_cb82adb2556ed13e(arg0) {
    const ret = getObject(arg0).getTime();
    return ret;
};

export function __wbg_new0_a57059d72c5b7aee() {
    const ret = new Date();
    return addHeapObject(ret);
};

export function __wbg_entries_65a76a413fc91037(arg0) {
    const ret = Object.entries(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_new_9962f939219f1820(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return __wbg_adapter_442(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = state0.b = 0;
    }
};

export function __wbg_reject_72477563edad55b7(arg0) {
    const ret = Promise.reject(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_resolve_99fe17964f31ffc0(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_then_11f7a54d67b4bfad(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};

export function __wbg_then_cedad20fbbd9418a(arg0, arg1, arg2) {
    const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};

export function __wbg_buffer_3f3d764d4747d564(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
};

export function __wbg_newwithbyteoffsetandlength_d9aa266703cb98be(arg0, arg1, arg2) {
    const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
    return addHeapObject(ret);
};

export function __wbg_new_8c3f0052272a457a(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
};

export function __wbg_set_83db9690f9353e79(arg0, arg1, arg2) {
    getObject(arg0).set(getObject(arg1), arg2 >>> 0);
};

export function __wbg_length_9e1ae1900cb0fbd5(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

export function __wbg_instanceof_Uint8Array_971eeda69eb75003(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Uint8Array;
    } catch {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_newwithlength_f5933855e4f48a19(arg0) {
    const ret = new Uint8Array(arg0 >>> 0);
    return addHeapObject(ret);
};

export function __wbg_subarray_58ad4efbb5bcb886(arg0, arg1, arg2) {
    const ret = getObject(arg0).subarray(arg1 >>> 0, arg2 >>> 0);
    return addHeapObject(ret);
};

export function __wbg_has_8359f114ce042f5a() { return handleError(function (arg0, arg1) {
    const ret = Reflect.has(getObject(arg0), getObject(arg1));
    return ret;
}, arguments) };

export function __wbg_set_bf3f89b92d5a34bf() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
    return ret;
}, arguments) };

export function __wbg_stringify_d6471d300ded9b68() { return handleError(function (arg0) {
    const ret = JSON.stringify(getObject(arg0));
    return addHeapObject(ret);
}, arguments) };

export function __wbindgen_bigint_get_as_i64(arg0, arg1) {
    const v = getObject(arg1);
    const ret = typeof(v) === 'bigint' ? v : undefined;
    getBigInt64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0n : ret;
    getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
};

export function __wbindgen_debug_string(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};

export function __wbindgen_throw(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

export function __wbindgen_memory() {
    const ret = wasm.memory;
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper851(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 164, __wbg_adapter_46);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2255(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 580, __wbg_adapter_49);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2446(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 643, __wbg_adapter_52);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2448(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 643, __wbg_adapter_52);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2450(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 643, __wbg_adapter_52);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2452(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 643, __wbg_adapter_52);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2660(arg0, arg1, arg2) {
    const ret = makeClosure(arg0, arg1, 738, __wbg_adapter_61);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2662(arg0, arg1, arg2) {
    const ret = makeClosure(arg0, arg1, 738, __wbg_adapter_64);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper2687(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 755, __wbg_adapter_67);
    return addHeapObject(ret);
};

export function __wbindgen_closure_wrapper3715(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 1177, __wbg_adapter_70);
    return addHeapObject(ret);
};

