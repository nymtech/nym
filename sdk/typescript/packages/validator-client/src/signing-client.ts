import {
  ExecuteResult,
  InstantiateOptions,
  InstantiateResult,
  MigrateResult,
  SigningCosmWasmClient,
  SigningCosmWasmClientOptions,
  UploadResult,
} from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet, EncodeObject } from '@cosmjs/proto-signing';
import { Coin, DeliverTxResponse, SignerData, StdFee } from '@cosmjs/stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import { ChangeAdminResult } from '@cosmjs/cosmwasm-stargate/build/signingcosmwasmclient';
import { TxRaw } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { nymGasPrice } from './stargate-helper';
import { IQueryClient } from './query-client';
import NyxdQuerier from './nyxd-querier';
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
import NymApiQuerier from './nym-api-querier';
import { makeBankMsgSend } from './utils';
import { ISimulateClient } from './types/simulate';

// methods exposed by `SigningCosmWasmClient`
export interface ICosmWasmSigning {
  simulate(signerAddress: string, messages: readonly EncodeObject[], memo: string | undefined): Promise<number>;

  upload(
    senderAddress: string,
    wasmCode: Uint8Array,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<UploadResult>;

  instantiate(
    senderAddress: string,
    codeId: number,
    msg: Record<string, unknown>,
    label: string,
    fee: StdFee | 'auto' | number,
    options?: InstantiateOptions,
  ): Promise<InstantiateResult>;

  updateAdmin(
    senderAddress: string,
    contractAddress: string,
    newAdmin: string,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ChangeAdminResult>;

  clearAdmin(
    senderAddress: string,
    contractAddress: string,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ChangeAdminResult>;

  migrate(
    senderAddress: string,
    contractAddress: string,
    codeId: number,
    migrateMsg: Record<string, unknown>,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<MigrateResult>;

  execute(
    senderAddress: string,
    contractAddress: string,
    msg: Record<string, unknown>,
    fee: StdFee | 'auto' | number,
    memo?: string,
    funds?: readonly Coin[],
  ): Promise<ExecuteResult>;

  sendTokens(
    senderAddress: string,
    recipientAddress: string,
    amount: readonly Coin[],
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<DeliverTxResponse>;

  delegateTokens(
    delegatorAddress: string,
    validatorAddress: string,
    amount: Coin,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<DeliverTxResponse>;

  undelegateTokens(
    delegatorAddress: string,
    validatorAddress: string,
    amount: Coin,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<DeliverTxResponse>;

  withdrawRewards(
    delegatorAddress: string,
    validatorAddress: string,
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<DeliverTxResponse>;

  signAndBroadcast(
    signerAddress: string,
    messages: readonly EncodeObject[],
    fee: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<DeliverTxResponse>;

  sign(
    signerAddress: string,
    messages: readonly EncodeObject[],
    fee: StdFee,
    memo: string,
    explicitSignerData?: SignerData,
  ): Promise<TxRaw>;
}

export interface INymSigning {
  clientAddress: string;
}

export interface ISigningClient extends IQueryClient, ICosmWasmSigning, INymSigning, ISimulateClient {
  bondMixNode(
    mixnetContractAddress: string,
    mixNode: MixNode,
    costParams: NodeCostParams,
    ownerSignature: string,
    pledge: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  unbondMixNode(mixnetContractAddress: string, fee?: StdFee | 'auto' | number, memo?: string): Promise<ExecuteResult>;

  bondGateway(
    mixnetContractAddress: string,
    gateway: Gateway,
    ownerSignature: string,
    pledge: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  unbondGateway(mixnetContractAddress: string, fee?: StdFee | 'auto' | number, memo?: string): Promise<ExecuteResult>;

  delegateToMixNode(
    mixnetContractAddress: string,
    mixId: number,
    amount: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  undelegateFromMixNode(
    mixnetContractAddress: string,
    mixId: number,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  updateMixnodeConfig(
    mixnetContractAddress: string,
    mixId: number,
    profitMarginPercent: number,
    fee: StdFee | 'auto' | number,
  ): Promise<ExecuteResult>;

  updateContractStateParams(
    mixnetContractAddress: string,
    newParams: ContractStateParams,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  // I don't see any point in exposing rewarding / vesting-related (INSIDE mixnet contract, like "BondMixnodeOnBehalf")
  // functionalities in our typescript client. However, if for some reason, we find we need them
  // they're rather trivial to add.
}

export default class SigningClient extends SigningCosmWasmClient implements ISigningClient {
  private nyxdQuerier: NyxdQuerier;

  private nymApiQuerier: NymApiQuerier;

  clientAddress: string;

  private constructor(
    clientAddress: string,
    nymApiUrl: string,
    tmClient: Tendermint34Client,
    wallet: DirectSecp256k1HdWallet,
    signerOptions: SigningCosmWasmClientOptions,
  ) {
    super(tmClient, wallet, signerOptions);
    this.clientAddress = clientAddress;
    this.nyxdQuerier = new NyxdQuerier(this);
    this.nymApiQuerier = new NymApiQuerier(nymApiUrl);
  }

  public static async connectWithNymSigner(
    wallet: DirectSecp256k1HdWallet,
    nyxdUrl: string,
    nymApiUrl: string,
    prefix: string,
    denom: string,
  ): Promise<SigningClient> {
    const [{ address }] = await wallet.getAccounts();
    const signerOptions: SigningCosmWasmClientOptions = {
      prefix,
      gasPrice: nymGasPrice(denom),
    };
    const tmClient = await Tendermint34Client.connect(nyxdUrl);
    return new SigningClient(address, nymApiUrl, tmClient, wallet, signerOptions);
  }

  // query related:

  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
    return this.nyxdQuerier.getContractVersion(mixnetContractAddress);
  }

  getMixNodeBonds(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeBondResponse> {
    return this.nyxdQuerier.getMixNodeBonds(mixnetContractAddress, limit, startAfter);
  }

  getMixNodesDetailed(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixNodeDetailsResponse> {
    return this.nyxdQuerier.getMixNodesDetailed(mixnetContractAddress, limit, startAfter);
  }

  getStakeSaturation(mixnetContractAddress: string, mixId: number) {
    return this.nyxdQuerier.getStakeSaturation(mixnetContractAddress, mixId);
  }

  getUnbondedMixNodeInformation(mixnetContractAddress: string, mixId: number): Promise<UnbondedMixnodeResponse> {
    return this.nyxdQuerier.getUnbondedMixNodeInformation(mixnetContractAddress, mixId);
  }

  getMixnodeRewardingDetails(mixnetContractAddress: string, mixId: number): Promise<MixNodeRewarding> {
    return this.nyxdQuerier.getMixnodeRewardingDetails(mixnetContractAddress, mixId);
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.nyxdQuerier.getGatewaysPaged(mixnetContractAddress, limit, startAfter);
  }

  getOwnedMixnode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.nyxdQuerier.getOwnedMixnode(mixnetContractAddress, address);
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.nyxdQuerier.ownsGateway(mixnetContractAddress, address);
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractState> {
    return this.nyxdQuerier.getStateParams(mixnetContractAddress);
  }

  getAllDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.getAllDelegationsPaged(mixnetContractAddress, limit, startAfter);
  }

  getUnbondedMixNodes(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedUnbondedMixnodesResponse> {
    return this.nyxdQuerier.getUnbondedMixNodes(mixnetContractAddress, limit, startAfter);
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mix_id: number,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.nyxdQuerier.getMixNodeDelegationsPaged(mixnetContractAddress, mix_id, limit, startAfter);
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.nyxdQuerier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter);
  }

  getDelegationDetails(mixnetContractAddress: string, mix_id: number, delegator: string): Promise<Delegation> {
    return this.nyxdQuerier.getDelegationDetails(mixnetContractAddress, mix_id, delegator);
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.nyxdQuerier.getLayerDistribution(mixnetContractAddress);
  }

  getRewardParams(mixnetContractAddress: string): Promise<RewardingParams> {
    return this.nyxdQuerier.getRewardParams(mixnetContractAddress);
  }

  getCachedGateways(): Promise<GatewayBond[]> {
    return this.nymApiQuerier.getCachedGateways();
  }

  getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getCachedMixnodes();
  }

  getActiveMixnodes(): Promise<MixNodeDetails[]> {
    return this.nymApiQuerier.getActiveMixnodes();
  }

  getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.nymApiQuerier.getRewardedMixnodes();
  }

  getSpendableCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<any> {
    return this.nyxdQuerier.getSpendableCoins(vestingContractAddress, vestingAccountAddress);
  }

  // signing related:

  bondMixNode(
    mixnetContractAddress: string,
    mixNode: MixNode,
    costParams: NodeCostParams,
    ownerSignature: string,
    pledge: Coin,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Bonding from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        bond_mixnode: {
          mix_node: mixNode,
          cost_params: costParams,
          owner_signature: ownerSignature,
        },
      },
      fee,
      memo,
      [pledge],
    );
  }

  unbondMixNode(
    mixnetContractAddress: string,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Unbonding from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        unbond_mixnode: {},
      },
      fee,
      memo,
    );
  }

  bondGateway(
    mixnetContractAddress: string,
    gateway: Gateway,
    ownerSignature: string,
    pledge: Coin,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default Gateway Bonding from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        bond_gateway: {
          gateway,
          owner_signature: ownerSignature,
        },
      },
      fee,
      memo,
      [pledge],
    );
  }

  unbondGateway(
    mixnetContractAddress: string,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default Gateway Unbonding from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        unbond_gateway: {},
      },
      fee,
      memo,
    );
  }

  delegateToMixNode(
    mixnetContractAddress: string,
    mixId: number,
    amount: Coin,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Delegation from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        delegate_to_mixnode: {
          mix_id: mixId,
        },
      },
      fee,
      memo,
      [amount],
    );
  }

  undelegateFromMixNode(
    mixnetContractAddress: string,
    mixId: number,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Undelegation from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        undelegate_from_mixnode: {
          mix_id: mixId,
        },
      },
      fee,
      memo,
    );
  }

  updateMixnodeConfig(
    mixnetContractAddress: string,
    mixId: number,
    profitMarginPercent: number,
    fee: StdFee | 'auto' | number,
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      { update_mixnode_config: { profit_margin_percent: profitMarginPercent, mix_id: mixId } },
      fee,
    );
  }

  updateContractStateParams(
    mixnetContractAddress: string,
    newParams: ContractStateParams,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default Contract State Params Update from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        update_contract_state_params: newParams,
      },
      fee,
      memo,
    );
  }

  // vesting related

  getVestingAccountsPaged(vestingContractAddress: string): Promise<VestingAccountsPaged> {
    return this.nyxdQuerier.getVestingAccountsPaged(vestingContractAddress);
  }

  getVestingAmountsAccountsPaged(vestingContractAddress: string): Promise<VestingAccountsCoinPaged> {
    return this.nyxdQuerier.getVestingAmountsAccountsPaged(vestingContractAddress);
  }

  getLockedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getLockedTokens(vestingContractAddress, vestingAccountAddress);
  }

  getSpendableTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getSpendableTokens(vestingContractAddress, vestingAccountAddress);
  }

  getVestedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getVestedTokens(vestingContractAddress, vestingAccountAddress);
  }

  getVestingTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getVestingTokens(vestingContractAddress, vestingAccountAddress);
  }

  getSpendableVestedTokens(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getSpendableVestedTokens(vestingContractAddress, vestingAccountAddress);
  }

  getSpendableRewards(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getSpendableRewards(vestingContractAddress, vestingAccountAddress);
  }

  getDelegatedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getDelegatedCoins(vestingContractAddress, vestingAccountAddress);
  }

  getPledgedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getPledgedCoins(vestingContractAddress, vestingAccountAddress);
  }

  getStakedCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getStakedCoins(vestingContractAddress, vestingAccountAddress);
  }

  getWithdrawnCoins(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getWithdrawnCoins(vestingContractAddress, vestingAccountAddress);
  }

  getStartTime(vestingContractAddress: string, vestingAccountAddress: string): Promise<string> {
    return this.nyxdQuerier.getStartTime(vestingContractAddress, vestingAccountAddress);
  }

  getEndTime(vestingContractAddress: string, vestingAccountAddress: string): Promise<string> {
    return this.nyxdQuerier.getEndTime(vestingContractAddress, vestingAccountAddress);
  }

  getOriginalVestingDetails(
    vestingContractAddress: string,
    vestingAccountAddress: string,
  ): Promise<OriginalVestingResponse> {
    return this.nyxdQuerier.getOriginalVestingDetails(vestingContractAddress, vestingAccountAddress);
  }

  getHistoricStakingRewards(vestingContractAddress: string, vestingAccountAddress: string): Promise<Coin> {
    return this.nyxdQuerier.getHistoricStakingRewards(vestingContractAddress, vestingAccountAddress);
  }

  getAccountDetails(vestingContractAddress: string, address: string): Promise<VestingAccountInfo> {
    return this.nyxdQuerier.getAccountDetails(vestingContractAddress, address);
  }

  getMixnode(vestingContractAddress: string, address: string): Promise<VestingAccountNode> {
    return this.nyxdQuerier.getMixnode(vestingContractAddress, address);
  }

  getGateway(vestingContractAddress: string, address: string): Promise<VestingAccountNode> {
    return this.nyxdQuerier.getGateway(vestingContractAddress, address);
  }

  getDelegationTimes(
    vestingContractAddress: string,
    mix_id: number,
    delegatorAddress: string,
  ): Promise<DelegationTimes> {
    return this.nyxdQuerier.getDelegationTimes(vestingContractAddress, mix_id, delegatorAddress);
  }

  getAllDelegations(vestingContractAddress: string): Promise<Delegations> {
    return this.nyxdQuerier.getAllDelegations(vestingContractAddress);
  }

  getDelegation(
    vestingContractAddress: string,
    vestingAccountAddress: string,
    mix_id: number,
  ): Promise<DelegationBlock> {
    return this.nyxdQuerier.getDelegation(vestingContractAddress, vestingAccountAddress, mix_id);
  }

  getTotalDelegationAmount(
    vestingContractAddress: string,
    vestingAccountAddress: string,
    mix_id: number,
    block_timestamp_sec: number,
  ): Promise<Coin> {
    return this.nyxdQuerier.getTotalDelegationAmount(
      vestingContractAddress,
      vestingAccountAddress,
      mix_id,
      block_timestamp_sec,
    );
  }

  getCurrentVestingPeriod(vestingContractAddress: string, address: string): Promise<Period> {
    return this.nyxdQuerier.getCurrentVestingPeriod(vestingContractAddress, address);
  }

  // simulation

  // TODO consider adding multipling factor

  simulateSend(signingAddress: string, from: string, to: string, amount: Coin[]) {
    const sendMsg = makeBankMsgSend(from, to, amount);
    return this.simulate(signingAddress, [sendMsg], 'simulate send tx');
  }
}
