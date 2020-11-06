Main = async () => {
    try {
        const { NymClient, set_panic_hook } = await import('@nymproject/nym-client-wasm');
        const directory = "https://qa-directory.nymtech.net";
        //"http://localhost:8080";
        client = new NymClient(directory);
    } catch (err) {
        console.log(`Unexpected error in loadWasm. [Message: ${err.message}]`);
    }
} 