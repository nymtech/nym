import NetClient, { INetClient } from "./net-client";
import { MixNode } from "./types";

export { ValidatorClient }

class ValidatorClient {

    url: string;
    mixNodes: MixNode[];
    mnemonic: string;
    netClient: INetClient;

    constructor(url: string, netClient: INetClient, mnemonic: string) {
        this.url = url;
        this.mixNodes = [];
        this.mnemonic = mnemonic;
        this.netClient = netClient;
    }

    connect(contractAddress: string, url: string) {
        NetClient.connect(contractAddress, this.mnemonic, url);
    }

    static loadMnemonic() { }

    static randomMnemonic() { }

    mnemonicToAddress() { }

    refreshMixNodes() { }

    send() { }

}