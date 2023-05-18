
import {
    ExtensionStorage,
    set_panic_hook
} from "@nymproject/storage-extension"

// // current limitation of rust-wasm for async stuff : (
// let client = null

async function main() {
    // // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    let storage = await new ExtensionStorage("my super duper password");

    const goodMnemonic = "figure aspect pill salute review sponsor army city muffin engine army kid rival chunk unit insect blouse paddle velvet shallow box crawl grace never"
    const badMnemonic = "foomp"

    let readEmpty = await storage.read_mnemonic("my-mnemonic1")
    console.log("value initial:", readEmpty);

    try {
        await storage.store_mnemonic("my-mnemonic1",  badMnemonic);
    } catch (e) {
        console.log("store error: ",e)
    }

    let anotherRead = await storage.read_mnemonic("my-mnemonic1")
    console.log("value bad store:", anotherRead);

    await storage.store_mnemonic("my-mnemonic1", goodMnemonic)

    let yetAnotherRead = await storage.read_mnemonic("my-mnemonic1")
    console.log("value good store:", yetAnotherRead);

    await storage.remove_mnemonic("my-mnemonic1")

    let finalRead = await storage.read_mnemonic("my-mnemonic1")
    console.log("value removed:", finalRead);

}


// Let's get started!
main();