import { contracts } from '@nymproject/contract-clients';
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";

async function main() {
  // generate a signer from a mnemonic
  const signer = await DirectSecp256k1HdWallet.fromMnemonic("...");
  const accounts = await signer.getAccounts();

  // make a signing client for the Nym Mixnet contract on mainnet
  const cosmWasmSigningClient = await SigningCosmWasmClient.connectWithSigner("https://rpc.nymtech.net:443", signer);
  const client = new contracts.Mixnet.MixnetClient(cosmWasmSigningClient, accounts[0].address, 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr');

  // delegate 1 NYM to mixnode with id 100
  const result = await client.delegateToMixnode({ mixId: 100 }, 'auto', undefined, [{ amount: `${1_000_000}`, denom: 'unym' }]);

  console.log(`Tx Hash = ${result.transactionHash}`);
}
