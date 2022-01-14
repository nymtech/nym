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
import NymdQuerier from './nymd-querier';
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
  MixOwnershipResponse,
  PagedAllDelegationsResponse,
  PagedDelegatorDelegationsResponse,
  PagedGatewayResponse,
  PagedMixDelegationsResponse,
  PagedMixnodeResponse,
  RewardingStatus,
} from './types';
import ValidatorApiQuerier from './validator-api-querier';

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

export interface ISigningClient extends IQueryClient, ICosmWasmSigning, INymSigning {
  bondMixNode(
    mixnetContractAddress: string,
    mixNode: MixNode,
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
    mixIdentity: string,
    amount: Coin,
    fee?: StdFee | 'auto' | number,
    memo?: string,
  ): Promise<ExecuteResult>;

  undelegateFromMixNode(
    mixnetContractAddress: string,
    mixIdentity: string,
    fee?: StdFee | 'auto' | number,
    memo?: string,
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
  private nymdQuerier: NymdQuerier;

  private validatorApiQuerier: ValidatorApiQuerier;

  clientAddress: string;

  private constructor(
    clientAddress: string,
    validatorApiUrl: string,
    tmClient: Tendermint34Client,
    wallet: DirectSecp256k1HdWallet,
    signerOptions: SigningCosmWasmClientOptions,
  ) {
    super(tmClient, wallet, signerOptions);
    this.clientAddress = clientAddress;
    this.nymdQuerier = new NymdQuerier(this);
    this.validatorApiQuerier = new ValidatorApiQuerier(validatorApiUrl);
  }

  public static async connectWithNymSigner(
    wallet: DirectSecp256k1HdWallet,
    nymdUrl: string,
    validatorApiUrl: string,
    prefix: string,
  ): Promise<SigningClient> {
    const [{ address }] = await wallet.getAccounts();
    const signerOptions: SigningCosmWasmClientOptions = {
      gasPrice: nymGasPrice(prefix),
    };
    const tmClient = await Tendermint34Client.connect(nymdUrl);
    return new SigningClient(address, validatorApiUrl, tmClient, wallet, signerOptions);
  }

  // query related:

  getContractVersion(mixnetContractAddress: string): Promise<MixnetContractVersion> {
    return this.nymdQuerier.getContractVersion(mixnetContractAddress);
  }

  getMixNodesPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedMixnodeResponse> {
    return this.nymdQuerier.getMixNodesPaged(mixnetContractAddress, limit, startAfter);
  }

  getGatewaysPaged(mixnetContractAddress: string, limit?: number, startAfter?: string): Promise<PagedGatewayResponse> {
    return this.nymdQuerier.getGatewaysPaged(mixnetContractAddress, limit, startAfter);
  }

  ownsMixNode(mixnetContractAddress: string, address: string): Promise<MixOwnershipResponse> {
    return this.nymdQuerier.ownsMixNode(mixnetContractAddress, address);
  }

  ownsGateway(mixnetContractAddress: string, address: string): Promise<GatewayOwnershipResponse> {
    return this.nymdQuerier.ownsGateway(mixnetContractAddress, address);
  }

  getStateParams(mixnetContractAddress: string): Promise<ContractStateParams> {
    return this.nymdQuerier.getStateParams(mixnetContractAddress);
  }

  getAllNetworkDelegationsPaged(
    mixnetContractAddress: string,
    limit?: number,
    startAfter?: [string, string],
  ): Promise<PagedAllDelegationsResponse> {
    return this.nymdQuerier.getAllNetworkDelegationsPaged(mixnetContractAddress, limit, startAfter);
  }

  getMixNodeDelegationsPaged(
    mixnetContractAddress: string,
    mixIdentity: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedMixDelegationsResponse> {
    return this.nymdQuerier.getMixNodeDelegationsPaged(mixnetContractAddress, mixIdentity, limit, startAfter);
  }

  getDelegatorDelegationsPaged(
    mixnetContractAddress: string,
    delegator: string,
    limit?: number,
    startAfter?: string,
  ): Promise<PagedDelegatorDelegationsResponse> {
    return this.nymdQuerier.getDelegatorDelegationsPaged(mixnetContractAddress, delegator, limit, startAfter);
  }

  getDelegationDetails(mixnetContractAddress: string, mixIdentity: string, delegator: string): Promise<Delegation> {
    return this.nymdQuerier.getDelegationDetails(mixnetContractAddress, mixIdentity, delegator);
  }

  getLayerDistribution(mixnetContractAddress: string): Promise<LayerDistribution> {
    return this.nymdQuerier.getLayerDistribution(mixnetContractAddress);
  }

  getRewardPool(mixnetContractAddress: string): Promise<string> {
    return this.nymdQuerier.getRewardPool(mixnetContractAddress);
  }

  getCirculatingSupply(mixnetContractAddress: string): Promise<string> {
    return this.nymdQuerier.getCirculatingSupply(mixnetContractAddress);
  }

  getEpochRewardPercent(mixnetContractAddress: string): Promise<number> {
    return this.nymdQuerier.getEpochRewardPercent(mixnetContractAddress);
  }

  getSybilResistancePercent(mixnetContractAddress: string): Promise<number> {
    return this.nymdQuerier.getSybilResistancePercent(mixnetContractAddress);
  }

  getRewardingStatus(
    mixnetContractAddress: string,
    mixIdentity: string,
    rewardingIntervalNonce: number,
  ): Promise<RewardingStatus> {
    return this.nymdQuerier.getRewardingStatus(mixnetContractAddress, mixIdentity, rewardingIntervalNonce);
  }

  getCachedGateways(): Promise<GatewayBond[]> {
    return this.validatorApiQuerier.getCachedGateways();
  }

  getCachedMixnodes(): Promise<MixNodeBond[]> {
    return this.validatorApiQuerier.getCachedMixnodes();
  }

  getActiveMixnodes(): Promise<MixNodeBond[]> {
    return this.validatorApiQuerier.getActiveMixnodes();
  }

  getRewardedMixnodes(): Promise<MixNodeBond[]> {
    return this.validatorApiQuerier.getRewardedMixnodes();
  }

  // signing related:

  bondMixNode(
    mixnetContractAddress: string,
    mixNode: MixNode,
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
    mixIdentity: string,
    amount: Coin,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Delegation from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        delegate_to_mixnode: {
          mix_identity: mixIdentity,
        },
      },
      fee,
      memo,
      [amount],
    );
  }

  undelegateFromMixNode(
    mixnetContractAddress: string,
    mixIdentity: string,
    fee: StdFee | 'auto' | number = 'auto',
    memo = 'Default MixNode Undelegation from Typescript',
  ): Promise<ExecuteResult> {
    return this.execute(
      this.clientAddress,
      mixnetContractAddress,
      {
        undelegate_from_mixnode: {
          mix_identity: mixIdentity,
        },
      },
      fee,
      memo,
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
}
