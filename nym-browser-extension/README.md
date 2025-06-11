# Nym Browser Extension Storage

A WebAssembly-based storage component for securely managing mnemonics in extensions. This component provides encrypted storage functionality using IndexedDB with password-based encryption.

## Overview

This storage component is built in Rust and compiled to WebAssembly, providing:

- **Secure mnemonic storage**: Password-encrypted storage of BIP39 mnemonics
- **IndexedDB integration**: Browser-native persistent storage
- **Multiple account support**: Store and manage multiple mnemonic phrases with custom names
- **Type-safe API**: Promise-based JavaScript API with proper error handling

## Getting Started

### Prerequisites

- Rust (latest stable)
- `wasm-pack` tool for building WebAssembly
- Node.js (for the demo server)

### Building

```bash
cd storage
make wasm-pack
```

This will compile the Rust code to WebAssembly and generate the necessary JavaScript bindings.

### Example Usage

See the [internal-dev example](./storage/internal-dev/index.js) for complete usage examples.

Basic usage:

```javascript
import init, { ExtensionStorage, set_panic_hook } from "@nymproject/extension-storage"

// Initialize the WASM module first
await init();

// Set up better error handling
set_panic_hook();

// Create storage instance with password
const storage = await new ExtensionStorage("your-secure-password");

// Store a mnemonic
const mnemonic = "your twenty four word mnemonic phrase goes here...";
await storage.store_mnemonic("my-wallet", mnemonic);

// Read a mnemonic
const retrievedMnemonic = await storage.read_mnemonic("my-wallet");

// Check if a mnemonic exists
const exists = await storage.has_mnemonic("my-wallet");

// Get all stored mnemonic keys
const allKeys = await storage.get_all_mnemonic_keys();

// Remove a mnemonic
await storage.remove_mnemonic("my-wallet");
```

## Development

To run the internal development example:

```bash
cd storage

make demo

# Option 2: Manual server setup
cd internal-dev && node serve.js

# Then open http://localhost:8000 in your browser
```

**Note**: The demo requires a server that properly serves WASM files with `application/wasm` MIME type. The Node.js server is recommended as it handles MIME types more reliably.

## API Reference

### Initialization
- `init()` - **Required**: Initialize the WASM module before using other functions

### Constructor
- `new ExtensionStorage(password: string)` - Creates a new storage instance with the given password

### Static Methods
- `ExtensionStorage.exists()` - Check if storage database exists

### Instance Methods
- `store_mnemonic(name: string, mnemonic: string)` - Store a mnemonic with the given name
- `read_mnemonic(name: string)` - Retrieve a mnemonic by name (returns null if not found)
- `has_mnemonic(name: string)` - Check if a mnemonic with the given name exists
- `get_all_mnemonic_keys()` - Get all stored mnemonic names
- `remove_mnemonic(name: string)` - Remove a mnemonic by name

### Error Handling
- `set_panic_hook()` - Set up better stack traces for Rust panics in development

## Security Features

- **Password-based encryption**: All data is encrypted using the provided password
- **BIP39 validation**: Mnemonics are validated before storage
- **Secure memory handling**: Sensitive data is zeroed from memory when no longer needed
- **Browser sandbox**: Runs within the browser's security model

## Architecture

The storage component consists of:

- **Rust core** (`src/storage.rs`): Main storage implementation with encryption
- **WASM bindings** (`src/lib.rs`): WebAssembly interface layer  
- **Error handling** (`src/error.rs`): Comprehensive error types
- **Build configuration** (`Cargo.toml`, `Makefile`): Build and dependency management

## Important Notes

1. **WASM Initialization**: Always call `await init()` before using any other functions
2. **MIME Types**: The demo requires a server that properly serves WASM files
3. **Browser Compatibility**: Requires modern browsers with WebAssembly support
4. **Module Loading**: Uses ES modules - ensure your build system supports them
