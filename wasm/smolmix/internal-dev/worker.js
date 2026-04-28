// smolmix-wasm Web Worker
//
// Loads the WASM module in a dedicated thread. All mixnet I/O
// (smoltcp polling, TLS crypto, DNS resolution, mixnet messaging)
// runs here, keeping the main thread responsive for UI rendering.
//
// Two communication channels:
//   1. Comlink — request/response for fetch (setupMixTunnel, mixFetch, disconnect)
//   2. Raw postMessage — bidirectional events for WebSocket (ws-connect, ws-send, ws-close)
//
// Comlink ignores messages without its internal UUID markers,
// so both channels coexist on the same Worker without conflicts.

import initWasm, {
  setupMixTunnel as wasmSetup,
  mixFetch as wasmFetch,
  disconnectMixTunnel as wasmDisconnect,
  mixSocket as wasmMixSocket,
  wsSend as wasmWsSend,
  wsClose as wasmWsClose,
} from "smolmix-wasm";
import * as Comlink from "comlink";

let wasmReady = false;

// Comlink API (fetch)

const api = {
  async setupMixTunnel(opts) {
    if (!wasmReady) {
      await initWasm();
      wasmReady = true;
    }
    await wasmSetup(opts);
  },

  async mixFetch(url, init) {
    return await wasmFetch(url, init || {});
  },

  async disconnectMixTunnel() {
    await wasmDisconnect();
  },
};

Comlink.expose(api);

// Raw postMessage API (WebSocket)

// Maps main-thread connId → WASM handleId
const wsConnMap = new Map();

self.addEventListener("message", async (event) => {
  const msg = event.data;
  if (!msg || typeof msg.kind !== "string") return;

  switch (msg.kind) {
    case "ws-connect": {
      if (!wasmReady) {
        self.postMessage({
          kind: "ws-event",
          connId: msg.connId,
          type: "error",
          data: "WASM not initialised — call setupMixTunnel first",
        });
        return;
      }

      // WASM callback: fires for open/text/binary/close/error events.
      // Captures connId so all events route to the correct MixSocket instance.
      const onEvent = (handleId, type, data) => {
        if (!wsConnMap.has(msg.connId)) {
          wsConnMap.set(msg.connId, handleId);
        }
        self.postMessage({ kind: "ws-event", connId: msg.connId, type, data });
      };

      try {
        await wasmMixSocket(msg.url, msg.protocols, onEvent);
      } catch (e) {
        console.error("[ws] connect failed:", e);
        self.postMessage({
          kind: "ws-event",
          connId: msg.connId,
          type: "error",
          data: String(e),
        });
      }
      break;
    }

    case "ws-send": {
      const handleId = wsConnMap.get(msg.connId);
      if (handleId == null) return;
      try {
        wasmWsSend(handleId, msg.payload);
      } catch (e) {
        self.postMessage({
          kind: "ws-event",
          connId: msg.connId,
          type: "error",
          data: String(e),
        });
      }
      break;
    }

    case "ws-close": {
      const handleId = wsConnMap.get(msg.connId);
      if (handleId == null) return;
      try {
        wasmWsClose(handleId, msg.code || 1000, msg.reason || "");
      } catch (e) {
        self.postMessage({
          kind: "ws-event",
          connId: msg.connId,
          type: "error",
          data: String(e),
        });
      }
      wsConnMap.delete(msg.connId);
      break;
    }
  }
});

self.postMessage({ kind: "Loaded" });
