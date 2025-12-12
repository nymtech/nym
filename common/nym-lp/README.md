# Nym Lewes Protocol

The Lewes Protocol (LP) is a secure network communication protocol implemented in Rust. This README provides an overview of the protocol's session management and replay protection mechanisms.

## Architecture Overview

```
+-----------------+     +----------------+     +---------------+
| Transport Layer |<--->| LP Session     |<--->| LP Codec      |
| (UDP/TCP)       |     | - Replay prot. |     | - Enc/dec only|
+-----------------+     | - Crypto state |     +---------------+
                        +----------------+
```

## Packet Structure

The protocol uses a structured packet format:

```
+------------------+-------------------+------------------+
| Header (16B)     | Message           | Trailer (16B)    |
| - Version (1B)   | - Type (2B)       | - Authentication |
| - Reserved (3B)  | - Content         | - tag/MAC        |
| - SenderIdx (4B) |                   |                  |
| - Counter (8B)   |                   |                  |
+------------------+-------------------+------------------+
```

- Header contains protocol version, sender identification, and counter for replay protection
- Message carries the actual payload with a type identifier
- Trailer provides authentication and integrity verification (16 bytes)
- Total packet size is constrained by MTU (1500 bytes), accounting for network overhead

## Sessions

- Sessions are managed by `LPSession` and `SessionManager` classes
- Each session has unique receiving and sending indices to identify connections
- Sessions maintain:
  - Cryptographic state (currently mocked implementations)
  - Counter for outgoing packets
  - Replay protection mechanism for incoming packets

## Session Management

- `SessionManager` handles session lifecycle (creation, retrieval, removal)
- Sessions are stored in a thread-safe HashMap indexed by receiving index
- The manager generates unique indices for new sessions
- Sessions are Arc-wrapped for safe concurrent access

## Replay Protection

- Implemented in the `ReceivingKeyCounterValidator` class
- Uses a bitmap-based approach to track received packet counters
- The bitmap allows reordering of up to 1024 packets (configurable)
- SIMD optimizations are used when available for performance

## Replay Protection Process

1. Quick validation (`will_accept_branchless`):
   - Checks if counter is valid before expensive operations
   - Detects duplicates, out-of-window packets
   
2. Marking packets (`mark_did_receive_branchless`):
   - Updates the bitmap to mark counter as received
   - Updates statistics and sliding window as needed

3. Window Sliding:
   - Automatically slides the acceptance window when new higher counters arrive
   - Clears bits for old counters that fall outside the window

This architecture effectively prevents replay attacks while allowing some packet reordering, an essential feature for secure network protocols. 