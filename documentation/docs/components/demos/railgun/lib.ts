// Railgun-over-the-mixnet logic, ported from wasm/railgun-demo/index.js.
// Two privacy layers: Nym hides the network (every RPC via mixFetch), Railgun
// hides the application layer (shielded notes break the on-chain graph).
//
// The @railgun-community SDK is imported dynamically (no bundled types here, so
// it is typed `any`) and the engine is a process-global singleton, so the
// started/loaded flags live at module scope, which is the faithful model.

import { HDNodeWallet, Mnemonic, JsonRpcProvider, keccak256, parseEther } from 'ethers';
import { withRetry } from '../shared/mixfetch';

export const SEPOLIA_CHAIN_ID = 11155111;
export const SEPOLIA_NETWORK_NAME = 'Ethereum_Sepolia';
export const SEPOLIA_WETH = '0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14';
export const TXID_VERSION_V2 = 'V2_PoseidonMerkle';
export const STORAGE_KEY = 'railgun-demo-mnemonic';
export const DEFAULT_MNEMONIC = 'inherit joy bubble reveal fit skin repair involve spoil cube robot angry';
const ENCRYPTION_KEY = '0101010101010101010101010101010101010101010101010101010101010101';

export type RailgunLog = (msg: string, colour?: 'green' | 'red' | 'orange' | 'gray') => void;
export interface RailgunWalletInfo { id: string; railgunAddress: string; }

let engineStarted = false;
let providerLoaded = false;

// Pure-client public address derivation. No network, no engine; works before
// the tunnel is up so the page can show the funding target immediately.
export function derivePublicAddress(phrase: string): HDNodeWallet {
  const mnemonic = Mnemonic.fromPhrase(phrase.trim());
  return HDNodeWallet.fromMnemonic(mnemonic, "m/44'/60'/0'/0/0");
}

async function ensureEngineStarted(): Promise<void> {
  if (engineStarted) return;
  const railgun: any = await import('@railgun-community/wallet');
  const { MemoryLevel }: any = await import('memory-level');
  const db = new MemoryLevel();
  // Read-only artifact store: Shield does not need proving artifacts.
  const artifactStore = new railgun.ArtifactStore(
    async () => null,
    async () => {},
    async () => false,
  );
  await railgun.startRailgunEngine(
    'railgundemo', // wallet source id; alphanumeric only
    db,
    false, // shouldDebug
    artifactStore,
    false, // useNativeArtifacts (Node-only)
    false, // skipMerkletreeScans
    undefined, // poiNodeURLs (undefined keeps POI uninstantiated)
    undefined, // customPOILists
    false, // verboseScanLogging
  );
  // Sepolia is POI-gated but the public aggregator is dead and POI is not what
  // this demo proves: clear the network's poi field so the engine treats it as
  // a pre-POI deployment. Production would point at a real POI URL instead.
  const { NETWORK_CONFIG }: any = await import('@railgun-community/shared-models');
  NETWORK_CONFIG[SEPOLIA_NETWORK_NAME].poi = undefined;
  // Disable GraphQL quick-sync (its subgraph map lacks Sepolia, so it would
  // spam-XHR /undefined). Falls back to direct eth_getLogs scanning.
  const engine = railgun.getEngine();
  engine.quickSyncEvents = async () => ({ commitmentEvents: [], unshieldEvents: [], nullifierEvents: [] });
  engine.quickSyncRailgunTransactionsV2 = async () => [];
  engineStarted = true;
}

async function loadProviderOnce(rpc: string): Promise<void> {
  if (providerLoaded) return;
  const railgun: any = await import('@railgun-community/wallet');
  // One provider, weight 2 (the validator requires totalWeight >= 2; a single
  // HTTPS endpoint avoids competing TCP handshakes during cold start).
  const fallbackConfig = { chainId: SEPOLIA_CHAIN_ID, providers: [{ provider: rpc, priority: 1, weight: 2 }] };
  // Third arg is Railgun's provider pollingInterval in ms, not a request timeout.
  await railgun.loadProvider(fallbackConfig, SEPOLIA_NETWORK_NAME, 10000);
  providerLoaded = true;
}

export async function ensureRailgunEngine(rpc: string, log: RailgunLog): Promise<void> {
  if (engineStarted && providerLoaded) return;
  log('initialising Railgun engine (one-time)...');
  await ensureEngineStarted();
  // loadProvider makes its first RPC calls over a cold mixnet route, paying the
  // TCP-connect and TLS-handshake cost; that first ethers request can time out,
  // so retry (the second attempt finds the connection pool warm).
  await withRetry(() => loadProviderOnce(rpc), 'loadProvider', { log });
  log('Railgun engine ready', 'green');
}

export async function createRailgunWalletFromMnemonic(phrase: string): Promise<RailgunWalletInfo> {
  const railgun: any = await import('@railgun-community/wallet');
  const creationBlockNumbers = { [SEPOLIA_NETWORK_NAME]: 10_900_000 };
  return await railgun.createRailgunWallet(ENCRYPTION_KEY, phrase, creationBlockNumbers);
}

// Shield ETH into a shielded note. The headline action: a 4-step flow that
// signs a shield key, estimates gas, populates the tx, then signs + broadcasts
// (idempotently) through the mixFetch-routed provider.
export async function shieldEth(opts: {
  publicWallet: HDNodeWallet;
  railgunWallet: RailgunWalletInfo;
  provider: JsonRpcProvider;
  amountStr: string;
  log: RailgunLog;
  onTxHash: (hash: string) => void;
}): Promise<void> {
  const { publicWallet, railgunWallet, provider, amountStr, log, onTxHash } = opts;
  const railgun: any = await import('@railgun-community/wallet');

  let amountWei: bigint;
  try {
    amountWei = parseEther(amountStr);
  } catch {
    log(`invalid amount: "${amountStr}"`, 'red');
    return;
  }
  if (amountWei <= 0n) {
    log('amount must be > 0', 'red');
    return;
  }

  log(`shielding ${amountStr} ETH -> ${railgunWallet.railgunAddress}`);

  // Step 1: shieldPrivateKey = keccak256 of a signature over a deterministic
  // message. Signing proves consent and binds the key to the public wallet.
  log('step 1/4: signing shield-key derivation message...');
  const msg = railgun.getShieldPrivateKeySignatureMessage();
  const sigHex = await publicWallet.signMessage(msg);
  const shieldPrivateKey = keccak256(sigHex);
  const wrappedERC20Amount = { tokenAddress: SEPOLIA_WETH, amount: amountWei };

  // Step 2: gas estimate (needs the funder's address to simulate the call).
  log('step 2/4: estimating gas via mixFetch...');
  // withRetry can't infer T from the `any`-typed SDK call, so it falls back to
  // unknown; annotate the result to read its fields.
  const gasEstResp: any = await withRetry(
    () =>
      railgun.gasEstimateForShieldBaseToken(
        TXID_VERSION_V2,
        SEPOLIA_NETWORK_NAME,
        railgunWallet.railgunAddress,
        shieldPrivateKey,
        wrappedERC20Amount,
        publicWallet.address,
      ),
    'gasEstimateForShieldBaseToken',
    { log },
  );
  log(`  gas estimate: ${gasEstResp.gasEstimate.toString()} units`);

  // EIP-1559 gas details, padded 50% so a transient spike during the mixnet
  // round trip does not strand the tx.
  const feeData = await provider.getFeeData();
  const maxFeePerGas = ((feeData.maxFeePerGas ?? feeData.gasPrice ?? 30_000_000_000n) * 3n) / 2n;
  const maxPriorityFeePerGas = ((feeData.maxPriorityFeePerGas ?? 2_000_000_000n) * 3n) / 2n;
  const gasDetails = { evmGasType: 2, gasEstimate: gasEstResp.gasEstimate, maxFeePerGas, maxPriorityFeePerGas };

  // Step 3: populate the actual transaction.
  log('step 3/4: populating shield transaction...');
  const populateResp = await railgun.populateShieldBaseToken(
    TXID_VERSION_V2,
    SEPOLIA_NETWORK_NAME,
    railgunWallet.railgunAddress,
    shieldPrivateKey,
    wrappedERC20Amount,
    gasDetails,
  );
  const tx = populateResp.transaction;

  // Step 4: sign then broadcast separately, so the tx hash is fixed before any
  // broadcast attempt and a dropped response can be retried idempotently.
  log('step 4/4: signing + broadcasting via mixFetch -> Nym...');
  const signer = publicWallet.connect(provider);
  const populated = await signer.populateTransaction(tx);
  const signedHex = await signer.signTransaction(populated);
  const txHash = keccak256(signedHex);
  log(`  signed tx hash: ${txHash}`);
  log(`  -> To: ${populated.to}  (Railgun Sepolia proxy contract)`);
  log(`  -> calldata selector: ${(populated.data || '').slice(0, 10)}  (Railgun shield function; Etherscan decodes this)`);
  log(`  -> full calldata: ${populated.data || ''}`);
  onTxHash(txHash);

  let sentTx: any;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      sentTx = await provider.broadcastTransaction(signedHex);
      log(`  broadcast OK (attempt ${attempt})`, 'green');
      break;
    } catch (e: any) {
      const m = e.shortMessage || e.message || String(e);
      try {
        const existing = await provider.getTransaction(txHash);
        if (existing) {
          log('  broadcast response failed but tx is on chain, partial success', 'green');
          sentTx = existing;
          break;
        }
      } catch {
        /* getTransaction failed too; treat as not on chain, retry */
      }
      if (attempt < 3) {
        log(`  broadcast attempt ${attempt}/3 failed (${m}), retrying in 10s...`, 'orange');
        await new Promise((r) => setTimeout(r, 10_000));
      } else {
        log('  broadcast failed after 3 attempts. The mixnet route to the RPC is degraded. Disconnect, tick "Use random IPR", reconnect, then Shield again.', 'red');
        throw e;
      }
    }
  }

  log('waiting for receipt (1 confirmation)...');
  const receipt = await sentTx.wait(1);
  if (receipt && receipt.status === 1) {
    log(`shielded. Block ${receipt.blockNumber}, gas used ${receipt.gasUsed}`, 'green');
    log("verify on Etherscan: the To field is Railgun's Sepolia proxy, the method decodes to a shield, and the logs hold an encrypted Shield commitment.", 'green');
  } else {
    log('tx mined but reverted', 'red');
  }
}
