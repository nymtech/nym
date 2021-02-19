import { MixNode } from "./types";

export { ValidatorClient }

class ValidatorClient {

    url: string;
    mixNodes: MixNode[];

    constructor(url: string) {
        this.url = url;
        this.mixNodes = [];
    }

    connect() { }

    loadMnemonic() { }

    randomMnemonic() { }

    mnemonicToAddress() { }

    refreshMixNodes() { }

    send() { }

}