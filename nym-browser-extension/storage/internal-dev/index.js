
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

    const anotherMnemonic = "salmon picture danger pill tomato hour hand chaos tray bargain frequent fuel scheme coil divert season lucky ginger mom stem mistake blanket lake suffer";
    const oneMore = "cat quiz circle letter trade unhappy quarter garlic sting gravity zone stock scatter merge account barrel forward fame club chest camp under crop connect"

    const key1 = "my-amazing-mnemonic"
    const key2 = "my-other-mnemonic"

    await storage.store_mnemonic(key1, anotherMnemonic)
    await storage.store_mnemonic(key2, oneMore)

    let allKeys = await storage.get_all_mnemonic_keys()
    console.log("keys:", allKeys)

    const anotherOne = "mammal fashion rice two marble high brain achieve first harsh infant timber flush cloud hunt address brand immune tip identify aspect call beyond once"
    const anotherKey = "some-mnemonic"

    let isPresent = await storage.has_mnemonic(anotherKey);
    console.log("has mnemonic: ", isPresent)

    await storage.store_mnemonic(anotherKey, anotherOne)

    let isPresentNew = await storage.has_mnemonic(anotherKey);
    console.log("has mnemonic: ", isPresentNew)

    await storage.remove_mnemonic(anotherKey)

    let isPresentEvenNewer =  await storage.has_mnemonic(anotherKey);
    console.log("has mnemonic: ", isPresentEvenNewer)

}


// Let's get started!
main();