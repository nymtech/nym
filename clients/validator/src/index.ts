import { Bip39, Random } from '@cosmjs/crypto';
import { DirectSecp256k1HdWallet, EncodeObject } from '@cosmjs/proto-signing';
import { coin as cosmosCoin, Coin, DeliverTxResponse, isDeliverTxFailure, StdFee } from '@cosmjs/stargate';
import {
  ExecuteResult,
  InstantiateOptions,
  InstantiateResult,
  MigrateResult,
  UploadResult,
} from '@cosmjs/cosmwasm-stargate';
import SigningClient, { ISigningClient } from './signing-client';
import {
  ContractStateParams,
  Delegation,
  Gateway,
  GatewayBond,
  MixnetContractVersion,
  MixNode,
  MixNodeBond,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixnodeResponse,
} from './types';
import {
  CoinMap,
  displayAmountToNative,
  MappedCoin,
  nativeCoinToDisplay,
  nativeToPrintable,
  printableBalance,
  printableCoin,
} from './currency';
import QueryClient from './query-client';
import { nymGasPrice } from './stargate-helper';

export { coins, coin } from '@cosmjs/stargate';
export { Coin };
export {
  displayAmountToNative,
  nativeCoinToDisplay,
  printableCoin,
  printableBalance,
  nativeToPrintable,
  MappedCoin,
  CoinMap,
};
export { nymGasPrice };

export interface INymClient {
  readonly mixnetContract: string;
  readonly vestingContract: string;
}

export default class ValidatorClient implements INymClient {
  readonly client: SigningClient | QueryClient;

  private readonly denom: string;

  private readonly prefix: string;

  readonly mixnetContract: string;

  readonly vestingContract: string;

  private constructor(
    client: SigningClient | QueryClient,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
  ) {
    this.client = client;
    this.prefix = prefix;
    this.denom = `u${prefix}`;

    this.mixnetContract = mixnetContract;
    this.vestingContract = vestingContract;
  }

  static async connect(
    mnemonic: string,
    nymdUrl: string,
    validatorApiUrl: string,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
  ): Promise<ValidatorClient> {
    const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);

    const signingClient = await SigningClient.connectWithNymSigner(wallet, nymdUrl, validatorApiUrl, prefix);
    return new ValidatorClient(signingClient, prefix, mixnetContract, vestingContract);
  }

  static async connectForQuery(
    nymdUrl: string,
    validatorApiUrl: string,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
  ): Promise<ValidatorClient> {
    const queryClient = await QueryClient.connectWithNym(nymdUrl, validatorApiUrl);
    return new ValidatorClient(queryClient, prefix, mixnetContract, vestingContract);
  }

  public get address(): string {
    if (this.client instanceof SigningClient) {
      return this.client.clientAddress;
    }
    return '';
  }

  private assertSigning() {
    if (this.client instanceof QueryClient) {
      throw new Error('Tried to perform signing action with a query client!');
    }
  }

  /**
   * Generates a random mnemonic, useful for creating new accounts.
   * @returns a fresh mnemonic.
   */
  static randomMnemonic(): string {
    return Bip39.encode(Random.getBytes(32)).toString();
  }

  /**
   * @param mnemonic A mnemonic from which to generate a public/private keypair.
   * @param prefix the bech32 address prefix (human readable part)
   * @returns the address for this client wallet
   */
  static async mnemonicToAddress(mnemonic: string, prefix: string): Promise<string> {
    const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);
    const [{ address }] = await wallet.getAccounts();
    return address;
  }

  static async buildWallet(mnemonic: string, prefix: string): Promise<DirectSecp256k1HdWallet> {
    const signerOptions = { prefix };
    return DirectSecp256k1HdWallet.fromMnemonic(mnemonic, signerOptions);
  }

  getBalance(address: string): Promise<Coin> {
    return this.client.getBalance(address, this.denom);
  }

  async getCachedGateways(): Promise<GatewayBond[]> {
    return this.client.getCachedGateways();
  }

  async getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.client.getCachedMixnodes();
  }

  async getActiveMixnodes(): Promise<MixNodeBond[]> {
    return this.client.getActiveMixnodes();
  }

  async getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.client.getRewardedMixnodes();
  }

  public async getMixnetContractSettings(): Promise<ContractStateParams> {
    return this.client.getStateParams(this.mixnetContract);
  }

  public async getMixnetContractVersion(): Promise<MixnetContractVersion> {
    return this.client.getContractVersion(this.mixnetContract);
  }

  public async getRewardPool(): Promise<string> {
    return this.client.getRewardPool(this.mixnetContract);
  }

  public async getCirculatingSupply(): Promise<string> {
    return this.client.getCirculatingSupply(this.mixnetContract);
  }

  public async getSybilResistancePercent(): Promise<number> {
    return this.client.getSybilResistancePercent(this.mixnetContract);
  }

  public async getIntervalRewardPercent(): Promise<number> {
    return this.client.getIntervalRewardPercent(this.mixnetContract);
  }

  public async getAllNymdMixnodes(): Promise<MixNodeBond[]> {
    let mixNodes: MixNodeBond[] = [];
    const limit = 50;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedMixnodeResponse = await this.client.getMixNodesPaged(this.mixnetContract, limit);
      mixNodes = mixNodes.concat(pagedResponse.nodes);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return mixNodes;
  }

  public async getAllNymdGateways(): Promise<GatewayBond[]> {
    let gateways: GatewayBond[] = [];
    const limit = 50;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedGatewayResponse = await this.client.getGatewaysPaged(this.mixnetContract, limit);
      gateways = gateways.concat(pagedResponse.nodes);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return gateways;
  }

  /**
   * Gets list of all delegations towards particular mixnode.
   *
   * @param mixIdentity identity of the node to which the delegation was sent
   */
  public async getAllNymdSingleMixnodeDelegations(mixIdentity: string): Promise<Delegation[]> {
    let delegations: Delegation[] = [];
    const limit = 250;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedMixDelegationsResponse = await this.client.getMixNodeDelegationsPaged(
        this.mixnetContract,
        mixIdentity,
        limit,
        startAfter,
      );
      delegations = delegations.concat(pagedResponse.delegations);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return delegations;
  }

  public async getAllNymdDelegatorDelegations(delegator: string): Promise<Delegation[]> {
    let delegations: Delegation[] = [];
    const limit = 250;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedDelegatorDelegationsResponse = await this.client.getDelegatorDelegationsPaged(
        this.mixnetContract,
        delegator,
        limit,
        startAfter,
      );
      delegations = delegations.concat(pagedResponse.delegations);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return delegations;
  }

  public async getAllNymdNetworkDelegations(): Promise<Delegation[]> {
    let delegations: Delegation[] = [];
    const limit = 250;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedAllDelegationsResponse = await this.client.getAllNetworkDelegationsPaged(
        this.mixnetContract,
        limit,
        startAfter,
      );
      delegations = delegations.concat(pagedResponse.delegations);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return delegations;
  }

  /**
   * Generate a minimum gateway bond required to create a fresh mixnode.
   *
   * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
   */
  public async minimumMixnodePledge(): Promise<Coin> {
    const stateParams = await this.getMixnetContractSettings();
    // we trust the contract to return a valid number
    return cosmosCoin(stateParams.minimum_mixnode_pledge, this.prefix);
  }

  /**
   * Generate a minimum gateway bond required to create a fresh gateway.
   *
   * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
   */
  public async minimumGatewayPledge(): Promise<Coin> {
    const stateParams = await this.getMixnetContractSettings();
    // we trust the contract to return a valid number
    return cosmosCoin(stateParams.minimum_gateway_pledge, this.prefix);
  }

  public async send(
    recipientAddress: string,
    coins: readonly Coin[],
    fee: StdFee | 'auto' | number = 'auto',
    memo?: string,
  ): Promise<DeliverTxResponse> {
    this.assertSigning();

    const result = await (this.client as ISigningClient).sendTokens(this.address, recipientAddress, coins, fee, memo);
    if (isDeliverTxFailure(result)) {
      throw new Error(
        `Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`,
      );
    }
    return result;
  }

  public async executeCustom(
    signerAddress: string,
    messages: readonly EncodeObject[],
    fee: StdFee | 'auto' | number = 'auto',
    memo?: string,
  ): Promise<DeliverTxResponse> {
    this.assertSigning();

    const result = await (this.client as ISigningClient).signAndBroadcast(signerAddress, messages, fee, memo);
    if (isDeliverTxFailure(result)) {
      throw new Error(
        `Error when broadcasting tx ${result.transactionHash} at height ${result.height}. Code: ${result.code}; Raw log: ${result.rawLog}`,
      );
    }
    return result;
  }

  public async upload(
    senderAddress: string,
    wasmCode: Uint8Array,
    fee: StdFee | 'auto' | number = 'auto',
    memo?: string,
  ): Promise<UploadResult> {
    this.assertSigning();
    return (this.client as ISigningClient).upload(senderAddress, wasmCode, fee, memo);
  }

  public async instantiate(
    senderAddress: string,
    codeId: number,
    initMsg: Record<string, unknown>,
    label: string,
    fee: StdFee | 'auto' | number = 'auto',
    options?: InstantiateOptions,
  ): Promise<InstantiateResult> {
    this.assertSigning();
    return (this.client as ISigningClient).instantiate(senderAddress, codeId, initMsg, label, fee, options);
  }

  public async migrate(
    senderAddress: string,
    contractAddress: string,
    codeId: number,
    migrateMsg: Record<string, unknown>,
    fee: StdFee | 'auto' | number = 'auto',
    memo?: string,
  ): Promise<MigrateResult> {
    this.assertSigning();
    return (this.client as ISigningClient).migrate(senderAddress, contractAddress, codeId, migrateMsg, fee, memo);
  }

  public async bondMixNode(
    mixNode: MixNode,
    ownerSignature: string,
    pledge: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).bondMixNode(this.mixnetContract, mixNode, ownerSignature, pledge, fee, memo);
  }

  public async unbondMixNode(fee?: StdFee | 'auto' | number, memo?: string): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).unbondMixNode(this.mixnetContract, fee, memo);
  }

  public async bondGateway(
    gateway: Gateway,
    ownerSignature: string,
    pledge: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).bondGateway(this.mixnetContract, gateway, ownerSignature, pledge, fee, memo);
  }

  public async unbondGateway(fee?: StdFee | 'auto' | number, memo?: string): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).unbondGateway(this.mixnetContract, fee, memo);
  }

  public async delegateToMixNode(
    mixIdentity: string,
    amount: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).delegateToMixNode(this.mixnetContract, mixIdentity, amount, fee, memo);
  }

  public async undelegateFromMixNode(
    mixIdentity: string,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    return (this.client as ISigningClient).undelegateFromMixNode(this.mixnetContract, mixIdentity, fee, memo);
  }

  public async updateMixnodeConfig(
    mixIdentity: string,
    fee: StdFee | 'auto' | number,
    profitPercentage: number,
  ): Promise<ExecuteResult> {
    return (this.client as ISigningClient).updateMixnodeConfig(this.mixnetContract, mixIdentity, profitPercentage, fee);
  }

  public async updateContractStateParams(
    newParams: ContractStateParams,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).updateContractStateParams(this.mixnetContract, newParams, fee, memo);
  }
}