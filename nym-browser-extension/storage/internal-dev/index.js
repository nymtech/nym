/**
 * Nym Extension Storage - Internal Development Example
 * 
 * This file demonstrates how to use the ExtensionStorage WASM module
 * for securely storing and managing BIP39 mnemonics in browser extensions.
 * 
 * To run this example:
 * 1. Build the WASM module: `cd .. && make wasm-pack`
 * 2. Serve this directory: `python3 -m http.server 8000`
 * 3. Open http://localhost:8000 in your browser
 * 4. Open the browser's developer console to see the output
 */

import init, {
    ExtensionStorage,
    set_panic_hook
} from "./wasm/extension_storage.js"

/**
 * Test mnemonics for demonstration
 * Note: These are for testing only - never use these in production!
 */
const TEST_MNEMONICS = {
    valid: {
        wallet1: "figure aspect pill salute review sponsor army city muffin engine army kid rival chunk unit insect blouse paddle velvet shallow box crawl grace never",
        wallet2: "salmon picture danger pill tomato hour hand chaos tray bargain frequent fuel scheme coil divert season lucky ginger mom stem mistake blanket lake suffer",
        wallet3: "cat quiz circle letter trade unhappy quarter garlic sting gravity zone stock scatter merge account barrel forward fame club chest camp under crop connect",
        wallet4: "mammal fashion rice two marble high brain achieve first harsh infant timber flush cloud hunt address brand immune tip identify aspect call beyond once"
    },
    invalid: "this is not a valid mnemonic phrase"
};

const STORAGE_PASSWORD = "my-super-secure-password-123";

/**
 * Helper function to log results with better formatting
 */
function logResult(operation, result, error = null) {
    const timestamp = new Date().toISOString();
    console.group(`🔍 [${timestamp}] ${operation}`);
    
    if (error) {
        console.error("❌ Error:", error);
    } else {
        console.log("✅ Result:", result);
    }
    
    console.groupEnd();
}

/**
 * Test basic storage operations
 */
async function testBasicOperations(storage) {
    console.log("\n📦 Testing Basic Storage Operations");
    console.log("=" .repeat(50));

    // Test storing a valid mnemonic
    try {
        await storage.store_mnemonic("test-wallet", TEST_MNEMONICS.valid.wallet1);
        logResult("Store valid mnemonic", "Successfully stored");
    } catch (error) {
        logResult("Store valid mnemonic", null, error);
    }

    // Test reading the stored mnemonic
    try {
        const result = await storage.read_mnemonic("test-wallet");
        logResult("Read stored mnemonic", result);
    } catch (error) {
        logResult("Read stored mnemonic", null, error);
    }

    // Test reading a non-existent mnemonic
    try {
        const result = await storage.read_mnemonic("non-existent");
        logResult("Read non-existent mnemonic", result);
    } catch (error) {
        logResult("Read non-existent mnemonic", null, error);
    }

    // Test storing an invalid mnemonic
    try {
        await storage.store_mnemonic("invalid-wallet", TEST_MNEMONICS.invalid);
        logResult("Store invalid mnemonic", "Should not reach here");
    } catch (error) {
        logResult("Store invalid mnemonic", null, error.toString());
    }

    // Clean up
    try {
        await storage.remove_mnemonic("test-wallet");
        logResult("Remove test wallet", "Successfully removed");
    } catch (error) {
        logResult("Remove test wallet", null, error);
    }
}

/**
 * Test multiple wallet management
 */
async function testMultipleWallets(storage) {
    console.log("\n👥 Testing Multiple Wallet Management");
    console.log("=" .repeat(50));

    // Store multiple wallets
    const walletNames = Object.keys(TEST_MNEMONICS.valid);
    const walletMnemonics = Object.values(TEST_MNEMONICS.valid);

    for (let i = 0; i < walletNames.length; i++) {
        try {
            await storage.store_mnemonic(walletNames[i], walletMnemonics[i]);
            logResult(`Store wallet: ${walletNames[i]}`, "Success");
        } catch (error) {
            logResult(`Store wallet: ${walletNames[i]}`, null, error);
        }
    }

    // Get all wallet keys
    try {
        const allKeys = await storage.get_all_mnemonic_keys();
        logResult("Get all wallet keys", allKeys);
    } catch (error) {
        logResult("Get all wallet keys", null, error);
    }

    // Check if specific wallets exist
    for (const walletName of walletNames) {
        try {
            const exists = await storage.has_mnemonic(walletName);
            logResult(`Check wallet exists: ${walletName}`, exists);
        } catch (error) {
            logResult(`Check wallet exists: ${walletName}`, null, error);
        }
    }

    // Read each wallet
    for (const walletName of walletNames) {
        try {
            const mnemonic = await storage.read_mnemonic(walletName);
            logResult(`Read wallet: ${walletName}`, `${mnemonic.substring(0, 20)}...`);
        } catch (error) {
            logResult(`Read wallet: ${walletName}`, null, error);
        }
    }
}

/**
 * Test wallet removal and cleanup
 */
async function testWalletRemoval(storage) {
    console.log("\n🗑️  Testing Wallet Removal");
    console.log("=" .repeat(50));

    const walletNames = Object.keys(TEST_MNEMONICS.valid);

    // Remove wallets one by one
    for (const walletName of walletNames) {
        try {
            await storage.remove_mnemonic(walletName);
            logResult(`Remove wallet: ${walletName}`, "Success");
        } catch (error) {
            logResult(`Remove wallet: ${walletName}`, null, error);
        }

        // Verify removal
        try {
            const exists = await storage.has_mnemonic(walletName);
            logResult(`Verify removal: ${walletName}`, `Exists: ${exists}`);
        } catch (error) {
            logResult(`Verify removal: ${walletName}`, null, error);
        }
    }

    // Check final state
    try {
        const allKeys = await storage.get_all_mnemonic_keys();
        logResult("Final wallet count", `${allKeys.length} wallets remaining`);
    } catch (error) {
        logResult("Final wallet count", null, error);
    }
}

/**
 * Test storage existence check
 */
async function testStorageExistence() {
    console.log("\n💾 Testing Storage Existence");
    console.log("=" .repeat(50));

    try {
        const exists = await ExtensionStorage.exists();
        logResult("Storage database exists", exists);
    } catch (error) {
        logResult("Storage database exists", null, error);
    }
}

/**
 * Main demonstration function
 */
async function runDemo() {
    console.log("🚀 Nym Extension Storage Demo");
    console.log("=" .repeat(60));
    console.log("This demo shows how to use the ExtensionStorage WASM module");
    console.log("Check the browser console for detailed output\n");

    try {
        // Initialize the WASM module
        console.log("🔧 Initializing WASM module...");
        await init();
        console.log("✅ WASM module initialized successfully\n");

        // Set up better stack traces for Rust panics
        set_panic_hook();
    } catch (error) {
        console.error("💥 Failed to initialize WASM module:", error);
        return;
    }

    // Test storage existence before creating instance
    await testStorageExistence();

    try {
        // Create storage instance with password
        console.log("🔐 Creating storage instance with password...");
        const storage = await new ExtensionStorage(STORAGE_PASSWORD);
        console.log("✅ Storage instance created successfully\n");

        // Run all tests
        await testBasicOperations(storage);
        await testMultipleWallets(storage);
        await testWalletRemoval(storage);

        // Test storage existence after operations
        await testStorageExistence();

        console.log("\n🎉 Demo completed successfully!");
        console.log("Check the IndexedDB in your browser's developer tools to see the stored data.");

    } catch (error) {
        console.error("💥 Fatal error during demo:", error);
    }
}

/**
 * Additional utility functions for interactive testing
 */
window.nymStorageDemo = {
    /**
     * Create a new storage instance for manual testing
     */
    async createStorage(password = STORAGE_PASSWORD) {
        try {
            await init();
            set_panic_hook();
            return await new ExtensionStorage(password);
        } catch (error) {
            console.error("Failed to initialize WASM or create storage:", error);
            throw error;
        }
    },

    /**
     * Quick test with a storage instance
     */
    async quickTest(storage, name = "test", mnemonic = TEST_MNEMONICS.valid.wallet1) {
        console.log(`Testing with wallet: ${name}`);
        
        await storage.store_mnemonic(name, mnemonic);
        console.log("✅ Stored");
        
        const retrieved = await storage.read_mnemonic(name);
        console.log("✅ Retrieved:", retrieved);
        
        const exists = await storage.has_mnemonic(name);
        console.log("✅ Exists:", exists);
        
        await storage.remove_mnemonic(name);
        console.log("✅ Removed");
        
        return "Test completed";
    },

    /**
     * Available test mnemonics
     */
    mnemonics: TEST_MNEMONICS
};

// Start the demo when the page loads
document.addEventListener('DOMContentLoaded', () => {
    console.log("📚 Interactive Demo Functions Available:");
    console.log("- window.nymStorageDemo.createStorage() - Create storage instance");
    console.log("- window.nymStorageDemo.quickTest(storage) - Run quick test");
    console.log("- window.nymStorageDemo.mnemonics - Test mnemonic phrases");
    console.log("\nStarting automated demo...\n");
    
    runDemo();
});

// Export for module usage
export { runDemo, TEST_MNEMONICS, STORAGE_PASSWORD };