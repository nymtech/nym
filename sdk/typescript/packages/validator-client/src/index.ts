import {
  ExecuteResult,
  InstantiateOptions,
  InstantiateResult,
  MigrateResult,
  UploadResult,
} from '@cosmjs/cosmwasm-stargate';
import { Bip39, Random } from '@cosmjs/crypto';
import { DirectSecp256k1HdWallet, EncodeObject } from '@cosmjs/proto-signing';
import { Coin, coin as cosmosCoin, DeliverTxResponse, isDeliverTxFailure, StdFee } from '@cosmjs/stargate';
import {
  ContractStateParams,
  Delegation,
  Gateway,
  GatewayBond,
  GatewayOwnershipResponse,
  LayerDistribution,
  MixnetContractVersion,
  MixNode,
  MixNodeBond,
  NodeCostParams,
  MixNodeDetails,
  MixNodeRewarding,
  MixOwnershipResponse,
  OriginalVestingResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixNodeBondResponse,
  PagedMixNodeDetailsResponse,
  PagedUnbondedMixnodesResponse,
  RewardingParams,
  StakeSaturationResponse,
  UnbondedMixnodeResponse,
  VestingAccountInfo,
  ContractState,
  VestingAccountsCoinPaged,
  VestingAccountsPaged,
  DelegationTimes,
  Delegations,
  Period,
  VestingAccountNode,
  DelegationBlock,
} from '@nymproject/types';
import QueryClient from './query-client';
import SigningClient, { ISigningClient } from './signing-client';

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

  readonly mainnetDenom = 'unym';

  readonly mainnetPrefix = 'n';

  private constructor(
    client: SigningClient | QueryClient,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
    denom: string,
  ) {
    this.client = client;
    this.prefix = prefix;
    this.denom = `u${denom}`;

    this.mixnetContract = mixnetContract;
    this.vestingContract = vestingContract;
  }

  static async connect(
    mnemonic: string,
    nyxdUrl: string,
    nymApiUrl: string,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
    denom: string,
  ): Promise<ValidatorClient> {
    const wallet = await ValidatorClient.buildWallet(mnemonic, prefix);

    const signingClient = await SigningClient.connectWithNymSigner(wallet, nyxdUrl, nymApiUrl, prefix, denom);
    return new ValidatorClient(signingClient, prefix, mixnetContract, vestingContract, denom);
  }

  static async connectForQuery(
    nyxdUrl: string,
    nymApiUrl: string,
    prefix: string,
    mixnetContract: string,
    vestingContract: string,
    denom: string,
  ): Promise<ValidatorClient> {
    const queryClient = await QueryClient.connectWithNym(nyxdUrl, nymApiUrl);
    return new ValidatorClient(queryClient, prefix, mixnetContract, vestingContract, denom);
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

  async getBalance(address: string): Promise<Coin> {
    return this.client.getBalance(address, this.denom);
  }

  async getCachedGateways(): Promise<GatewayBond[]> {
    return this.client.getCachedGateways();
  }

  async getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.client.getCachedMixnodes();
  }

  async getStakeSaturation(mixId: number): Promise<StakeSaturationResponse> {
    return this.client.getStakeSaturation(this.mixnetContract, mixId);
  }

  async getActiveMixnodes(): Promise<MixNodeDetails[]> {
    return this.client.getActiveMixnodes();
  }

  async getUnbondedMixNodeInformation(mixId: number): Promise<UnbondedMixnodeResponse> {
    return this.client.getUnbondedMixNodeInformation(this.mixnetContract, mixId);
  }

  async getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.client.getRewardedMixnodes();
  }

  async getMixnodeRewardingDetails(mixId: number): Promise<MixNodeRewarding> {
    return this.client.getMixnodeRewardingDetails(this.mixnetContract, mixId);
  }

  async getOwnedMixnode(address: string): Promise<MixOwnershipResponse> {
    return this.client.getOwnedMixnode(this.mixnetContract, address);
  }

  async ownsGateway(address: string): Promise<GatewayOwnershipResponse> {
    return this.client.ownsGateway(this.mixnetContract, address);
  }

  async getLayerDistribution(): Promise<LayerDistribution> {
    return this.client.getLayerDistribution(this.mixnetContract);
  }

  public async getMixnetContractSettings(): Promise<ContractState> {
    return this.client.getStateParams(this.mixnetContract);
  }

  public async getMixnetContractVersion(): Promise<MixnetContractVersion> {
    return this.client.getContractVersion(this.mixnetContract);
  }

  public async getVestingContractVersion(): Promise<MixnetContractVersion> {
    return this.client.getContractVersion(this.vestingContract);
  }

  public async getSpendableCoins(vestingAccountAddress: string): Promise<MixnetContractVersion> {
    return this.client.getSpendableCoins(this.vestingContract, vestingAccountAddress);
  }

  public async getRewardParams(): Promise<RewardingParams> {
    return this.client.getRewardParams(this.mixnetContract);
  }

  async getUnbondedMixNodes(): Promise<UnbondedMixnodeResponse[]> {
    let mixNodes: UnbondedMixnodeResponse[] = [];
    const limit = 50;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedUnbondedMixnodesResponse = await this.client.getUnbondedMixNodes(
        this.mixnetContract,
        limit,
        startAfter,
      );

      mixNodes = mixNodes.concat(pagedResponse.nodes);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return mixNodes;
  }

  public async getMixNodeBonds(): Promise<MixNodeBond[]> {
    let mixNodes: MixNodeBond[] = [];
    const limit = 50;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedMixNodeBondResponse = await this.client.getMixNodeBonds(
        this.mixnetContract,
        limit,
        startAfter,
      );
      mixNodes = mixNodes.concat(pagedResponse.nodes);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return mixNodes;
  }

  public async getMixNodesDetailed(): Promise<MixNodeDetails[]> {
    let mixNodes: MixNodeDetails[] = [];
    const limit = 50;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedMixNodeDetailsResponse = await this.client.getMixNodesDetailed(
        this.mixnetContract,
        limit,
        startAfter,
      );
      mixNodes = mixNodes.concat(pagedResponse.nodes);
      startAfter = pagedResponse.start_next_after;
      // if `start_next_after` is not set, we're done
      if (!startAfter) {
        break;
      }
    }

    return mixNodes;
  }

  public async getAllNyxdGateways(): Promise<GatewayBond[]> {
    const pagedResponse: PagedGatewayResponse = await this.client.getGatewaysPaged(this.mixnetContract);
    return pagedResponse.nodes;
  }

  /**
   * Gets list of all delegations towards particular mixnode.
   *
   * @param mix_id identity of the node to which the delegation was sent
   */
  public async getAllNyxdSingleMixnodeDelegations(mix_id: number): Promise<Delegation[]> {
    let delegations: Delegation[] = [];
    const limit = 250;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedMixDelegationsResponse = await this.client.getMixNodeDelegationsPaged(
        this.mixnetContract,
        mix_id,
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

  public async getAllNyxdDelegatorDelegations(delegator: string): Promise<Delegation[]> {
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

  public async getAllNyxdDelegations(): Promise<Delegation[]> {
    let delegations: Delegation[] = [];
    const limit = 250;
    let startAfter;
    for (;;) {
      // eslint-disable-next-line no-await-in-loop
      const pagedResponse: PagedAllDelegationsResponse = await this.client.getAllDelegationsPaged(
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

  public async getDelegationDetails(mix_id: number, delegator: string): Promise<Delegation> {
    return this.client.getDelegationDetails(this.mixnetContract, mix_id, delegator);
  }

  /**
   * Generate a minimum gateway bond required to create a fresh mixnode.
   *
   * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
   */
  public async minimumMixnodePledge(): Promise<Coin> {
    const stateParams = await this.getMixnetContractSettings();
    // we trust the contract to return a valid number
    return cosmosCoin(stateParams.params.minimum_mixnode_pledge, this.prefix);
  }

  /**
   * Generate a minimum gateway bond required to create a fresh gateway.
   *
   * @returns a `Coin` instance containing minimum amount of coins to pledge a gateway.
   */
  public async minimumGatewayPledge(): Promise<Coin> {
    const stateParams = await this.getMixnetContractSettings();
    // we trust the contract to return a valid number
    return cosmosCoin(stateParams.params.minimum_gateway_pledge, this.prefix);
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
    costParams: NodeCostParams,
    pledge: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).bondMixNode(
      this.mixnetContract,
      mixNode,
      costParams,
      ownerSignature,
      pledge,
      fee,
      memo,
    );
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
    mixId: number,
    amount: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).delegateToMixNode(this.mixnetContract, mixId, amount, fee, memo);
  }

  public async undelegateFromMixNode(
    mixId: number,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    return (this.client as ISigningClient).undelegateFromMixNode(this.mixnetContract, mixId, fee, memo);
  }

  public async updateMixnodeConfig(
    mixId: number,
    fee: StdFee | 'auto' | number,
    profitPercentage: number,
  ): Promise<ExecuteResult> {
    return (this.client as ISigningClient).updateMixnodeConfig(this.mixnetContract, mixId, profitPercentage, fee);
  }

  public async updateContractStateParams(
    newParams: ContractStateParams,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult> {
    this.assertSigning();
    return (this.client as ISigningClient).updateContractStateParams(this.mixnetContract, newParams, fee, memo);
  }

  // VESTING
  // TODO - MOVE TO A DIFFERENT FILE

  public async getVestingAccountsPaged(): Promise<VestingAccountsPaged> {
    return this.client.getVestingAccountsPaged(this.vestingContract);
  }

  public async getVestingAmountsAccountsPaged(): Promise<VestingAccountsCoinPaged> {
    return this.client.getVestingAmountsAccountsPaged(this.vestingContract);
  }

  public async getLockedTokens(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getLockedTokens(this.vestingContract, vestingAccountAddress);
  }

  public async getSpendableTokens(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getSpendableTokens(this.vestingContract, vestingAccountAddress);
  }

  public async getVestedTokens(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getVestedTokens(this.vestingContract, vestingAccountAddress);
  }

  public async getVestingTokens(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getVestingTokens(this.vestingContract, vestingAccountAddress);
  }

  public async getSpendableVestedTokens(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getSpendableVestedTokens(this.vestingContract, vestingAccountAddress);
  }

  public async getSpendableRewards(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getSpendableRewards(this.vestingContract, vestingAccountAddress);
  }

  public async getDelegatedCoins(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getDelegatedCoins(this.vestingContract, vestingAccountAddress);
  }

  public async getPledgedCoins(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getPledgedCoins(this.vestingContract, vestingAccountAddress);
  }

  public async getStakedCoins(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getStakedCoins(this.vestingContract, vestingAccountAddress);
  }

  public async getWithdrawnCoins(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getWithdrawnCoins(this.vestingContract, vestingAccountAddress);
  }

  public async getStartTime(vestingAccountAddress: string): Promise<string> {
    return this.client.getStartTime(this.vestingContract, vestingAccountAddress);
  }

  public async getEndTime(vestingAccountAddress: string): Promise<string> {
    return this.client.getEndTime(this.vestingContract, vestingAccountAddress);
  }

  public async getOriginalVestingDetails(vestingAccountAddress: string): Promise<OriginalVestingResponse> {
    return this.client.getOriginalVestingDetails(this.vestingContract, vestingAccountAddress);
  }

  public async getHistoricStakingRewards(vestingAccountAddress: string): Promise<Coin> {
    return this.client.getHistoricStakingRewards(this.vestingContract, vestingAccountAddress);
  }

  public async getAccountDetails(address: string): Promise<VestingAccountInfo> {
    return this.client.getAccountDetails(this.vestingContract, address);
  }

  public async getMixnode(address: string): Promise<VestingAccountNode> {
    return this.client.getMixnode(this.vestingContract, address);
  }

  public async getGateway(address: string): Promise<VestingAccountNode> {
    return this.client.getGateway(this.vestingContract, address);
  }

  public async getDelegationTimes(mix_id: number, delegatorAddress: string): Promise<DelegationTimes> {
    return this.client.getDelegationTimes(this.vestingContract, mix_id, delegatorAddress);
  }

  public async getAllDelegations(): Promise<Delegations> {
    return this.client.getAllDelegations(this.vestingContract);
  }

  public async getDelegation(address: string, mix_id: number): Promise<DelegationBlock> {
    return this.client.getDelegation(this.vestingContract, address, mix_id);
  }

  public async getTotalDelegationAmount(address: string, mix_id: number, block_timestamp_sec: number): Promise<Coin> {
    return this.client.getTotalDelegationAmount(this.vestingContract, address, mix_id, block_timestamp_sec);
  }

  public async getCurrentVestingPeriod(address: string): Promise<Period> {
    return this.client.getCurrentVestingPeriod(this.vestingContract, address);
  }

  // SIMULATE

  public async simulateSend({
    signingAddress,
    from,
    to,
    amount,
  }: {
    signingAddress: string;
    from: string;
    to: string;
    amount: Coin[];
  }) {
    return (this.client as ISigningClient).simulateSend(signingAddress, from, to, amount);
  }
}
