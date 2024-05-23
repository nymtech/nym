// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const RUST_WASM_URL = "zknym_lib_bg.wasm"

importScripts('zknym_lib.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    set_panic_hook,
    prepareBlindSign,
    blindSign,
    setup,
    signSimple,
    verifySimple,
    keygen,
    simpleRandomiseCredential,
    ttpKeygen,
    aggregateVerificationKeyShares,
    aggregateSignatureShares,
    proveBandwidthCredential,
    verifyCredential,
    NymIssuanceBandwidthVoucher
} = wasm_bindgen;


// I've made it before starting on VPN API so didn't know exact requirements
function simpleGenericUsage() {
    // SIMPLE SETUP: no threshold, no private attributes
    console.log(">>> simple example...")
    setup({numAttributes: 4, setGlobal: true})
    let keypair = keygen()

    let vk = keypair.verificationKey();

    let goodAttributes = ["foomp", "foo", "bar"]
    let badAttributes = ["I", "didn't", "sign", "that"]

    let sig_simple = signSimple(goodAttributes, keypair)
    console.log("signature:", sig_simple.stringify());

    let verified = verifySimple(vk, goodAttributes, sig_simple)
    let verified2 = verifySimple(vk, badAttributes, sig_simple)

    let randomised = simpleRandomiseCredential(sig_simple);
    let verified3 = verifySimple(vk, goodAttributes, randomised)

    console.log(`verified1: ${verified} (good attributes), verified2: ${verified2} (bad attributes), verified3: ${verified3} (good attributes + randomised)`)


    // 'NORMAL' SETUP: threshold with private attributes:
    console.log(">>> full example...")
    let t3_5_keys = ttpKeygen({threshold: 3, authorities: 5})

    // currently they HAVE TO correspond to our bandwidth credential attributes,
    // i.e. serial number and binding number, because it seems we removed generic verification methods
    // ¯\_(ツ)_/¯
    let serialNumber = "credential-serial-number"
    let bindingNumber = "credential-binding-number"
    let privateAttributes = [serialNumber, bindingNumber]
    let publicAttributes = ["foo", "bar"]

    // sign the attributes
    console.log("creating partial credentials...");
    let blindSignRequestData = prepareBlindSign(privateAttributes, publicAttributes)
    let blindSignRequest = blindSignRequestData.blindSignRequest()
    let blindPartial1 = blindSign(t3_5_keys[0], blindSignRequest, publicAttributes)
    let blindPartial2 = blindSign(t3_5_keys[1], blindSignRequest, publicAttributes)
    let blindPartial3 = blindSign(t3_5_keys[2], blindSignRequest, publicAttributes)
    let blindPartial4 = blindSign(t3_5_keys[3], blindSignRequest, publicAttributes)
    let blindPartial5 = blindSign(t3_5_keys[4], blindSignRequest, publicAttributes)

    console.log("unblinding signatures...");
    let pedersenOpenings = blindSignRequestData.pedersenCommitmentsOpenings()
    let partialSig1 = blindPartial1.unblind(t3_5_keys[0].verificationKey(), pedersenOpenings)
    let partialSig2 = blindPartial2.unblind(t3_5_keys[1].verificationKey(), pedersenOpenings)
    let partialSig3 = blindPartial3.unblind(t3_5_keys[2].verificationKey(), pedersenOpenings)
    let partialSig4 = blindPartial4.unblind(t3_5_keys[3].verificationKey(), pedersenOpenings)
    let partialSig5 = blindPartial5.unblind(t3_5_keys[4].verificationKey(), pedersenOpenings)

    // aggregate signature:
    console.log("aggregating signatures...")
    let sigShare1 = partialSig1.intoShare(t3_5_keys[0].index())
    let sigShare2 = partialSig2.intoShare(t3_5_keys[1].index())
    let sigShare3 = partialSig3.intoShare(t3_5_keys[2].index())
    let sigShare4 = partialSig4.intoShare(t3_5_keys[3].index())
    let sigShare5 = partialSig5.intoShare(t3_5_keys[4].index())
    let masterCred1 = aggregateSignatureShares([sigShare1.cloneDataPointer(), sigShare3, sigShare4])
    let masterCred2 = aggregateSignatureShares([sigShare1, sigShare2, sigShare5])

    // key shares:
    console.log("aggregating verification keys...");
    let vk1 = t3_5_keys[0].verificationKeyShare()
    let vk2 = t3_5_keys[1].verificationKeyShare()
    let vk3 = t3_5_keys[2].verificationKeyShare()
    let vk4 = t3_5_keys[3].verificationKeyShare()
    let vk5 = t3_5_keys[4].verificationKeyShare()

    // master verification key:
    let masterVk1 = aggregateVerificationKeyShares([vk1, vk2.cloneDataPointer(), vk3])
    let masterVk2 = aggregateVerificationKeyShares([vk2, vk4, vk5])

    // attempt to 'spend'/'prove' the credential (note that the master keys and credentials should be cryptographically identical):
    console.log("attempting to spend the credential...");
    let verifyReq1 = proveBandwidthCredential(masterVk1, masterCred2, serialNumber, bindingNumber)
    let verifyReq2 = proveBandwidthCredential(masterVk2, masterCred1, serialNumber, bindingNumber)

    console.log("verifying the credential...");
    let verifiedMaster1 = verifyCredential(masterVk1, verifyReq1, publicAttributes)
    let verifiedMaster2 = verifyCredential(masterVk1, verifyReq2, publicAttributes)

    console.log(`verified1: ${verifiedMaster1}, verified2: ${verifiedMaster2}`)
}

async function frontendSimulation() {
    console.log("getting opts");
    const res = await fetch("http://localhost:8080/api/v1/bandwidth-voucher/prehashed-public-attributes", {
        headers: new Headers({'Authorization': 'Bearer foomp'})
    });
    const opts = await res.json()

    const issuanceVoucher = new NymIssuanceBandwidthVoucher(opts)
    const blindSignRequest = issuanceVoucher.getBlindSignRequest()

    console.log("getting partial vks");
    const partialVksRes = await fetch("http://localhost:8080/api/v1/bandwidth-voucher/partial-verification-keys", {
        headers: new Headers({'Authorization': 'Bearer foomp'})
    });
    const partialVks = await partialVksRes.json()

    console.log("getting master vk")
    const masterVkRes = await fetch("http://localhost:8080/api/v1/bandwidth-voucher/master-verification-key", {
        headers: new Headers({'Authorization': 'Bearer foomp'})
    });
    const masterVk = await masterVkRes.json()

    console.log("getting blinded shares")
    const sharesRes = await fetch("http://localhost:8080/api/v1/bandwidth-voucher/obtain", {
        method: "POST",
        headers: new Headers(
            {
                'Authorization': 'Bearer foomp',
                "Content-Type": "application/json",
            }
        ),
        body: JSON.stringify({
            blindSignRequest
        })
    });

    const credentialShares = await sharesRes.json()

    console.log("unblinding shares");
    const bandwidthVoucher = issuanceVoucher.unblindShares(credentialShares, partialVks)
    console.log("is valid: ", bandwidthVoucher.ensureIsValid(masterVk.bs58EncodedKey))

    const serialised = bandwidthVoucher.serialise();
    console.log("serialised:\n", serialised)
}

async function main() {
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN START");

    // load rust WASM package
    await wasm_bindgen(RUST_WASM_URL);
    console.log('Loaded RUST WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();


    await frontendSimulation()


    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}


// Let's get started!
main();