import React, { useContext, useEffect, useReducer } from 'react';
import { Box, Button, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { TransactionExecuteResult } from '@nymproject/types';
import { ErrorOutline } from '@mui/icons-material';
import { ConfirmationModal, NymCard } from '../../../components';
import NodeIdentityModal from './NodeIdentityModal';
import {
  ACTIONTYPE,
  BondState,
  BondStatus,
  FormStep,
  GatewayAmount,
  GatewayData,
  MixnodeAmount,
  MixnodeData,
  NodeData,
} from '../types';
import AmountModal from './AmountModal';
import { AppContext, urls } from '../../../context';
import SummaryModal from './SummaryModal';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  vestingBondGateway,
  vestingBondMixNode,
} from '../../../requests';
import { useGetFee } from '../../../hooks/useGetFee';

const initialState: BondState = {
  showModal: false,
  formStep: 1,
  bondStatus: 'init',
};

function reducer(state: BondState, action: ACTIONTYPE) {
  let step;
  switch (action.type) {
    case 'change_bond_type':
      return { ...state, type: action.payload };
    case 'set_node_data':
      return { ...state, nodeData: action.payload };
    case 'set_amount_data':
      return { ...state, amountData: action.payload };
    case 'set_step':
      return { ...state, formStep: action.payload };
    case 'set_tx':
      return { ...state, tx: action.payload };
    case 'set_bond_status':
      return { ...state, bondStatus: action.payload };
    case 'set_error':
      return { ...state, error: action.payload, bondStatus: 'error' as BondStatus };
    case 'next_step':
      step = state.formStep + 1;
      return { ...state, formStep: step <= 4 ? (step as FormStep) : 4 };
    case 'prev_step':
      step = state.formStep - 1;
      return { ...state, formStep: step >= 1 ? (step as FormStep) : 1 };
    case 'show_modal':
      return { ...state, showModal: true };
    case 'close_modal':
      return { ...state, showModal: false };
    case 'reset':
      return initialState;
    default:
      throw new Error();
  }
}

const BondingCard = () => {
  const [state, dispatch] = useReducer(reducer, initialState);
  const { formStep, showModal } = state;

  const { userBalance, clientDetails, network } = useContext(AppContext);
  const { fee } = useGetFee();

  useEffect(() => {
    dispatch({ type: 'reset' });
  }, [clientDetails]);

  const bondMixnode = async () => {
    let tx: TransactionExecuteResult | undefined;
    const { signature, identityKey, sphinxKey, host, version, mixPort, verlocPort, httpApiPort } =
      state.nodeData as NodeData<MixnodeData>;
    const { profitMargin, amount, tokenPool } = state.amountData as MixnodeAmount;
    const payload = {
      ownerSignature: signature,
      mixnode: {
        identity_key: identityKey,
        sphinx_key: sphinxKey,
        host,
        version,
        mix_port: mixPort,
        profit_margin_percent: profitMargin,
        verloc_port: verlocPort,
        http_api_port: httpApiPort,
      },
      pledge: amount,
      fee: fee?.fee,
    };
    if (tokenPool !== 'locked' && tokenPool !== 'balance') {
      throw new Error(`token pool [${tokenPool}] not supported`);
    }
    try {
      if (tokenPool === 'balance') {
        tx = await bondMixNodeRequest(payload);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondMixNode(payload);
        await userBalance.fetchTokenAllocation();
      }
      dispatch({ type: 'set_bond_status', payload: 'success' });
      return tx;
    } catch (e: any) {
      dispatch({ type: 'set_error', payload: e });
    }
    return undefined;
  };

  const bondGateway = async () => {
    let tx: TransactionExecuteResult | undefined;
    const { signature, identityKey, sphinxKey, host, version, location, mixPort, clientsPort } =
      state.nodeData as NodeData<GatewayData>;
    const { amount, tokenPool } = state.amountData as GatewayAmount;
    const payload = {
      ownerSignature: signature,
      gateway: {
        identity_key: identityKey,
        sphinx_key: sphinxKey,
        host,
        version,
        mix_port: mixPort,
        location,
        clients_port: clientsPort,
      },
      pledge: amount,
      fee: fee?.fee,
    };
    try {
      if (tokenPool === 'balance') {
        tx = await bondGatewayRequest(payload);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondGateway(payload);
        await userBalance.fetchTokenAllocation();
      }
      dispatch({ type: 'set_bond_status', payload: 'success' });
      return tx;
    } catch (e: any) {
      dispatch({ type: 'set_error', payload: e });
    }
    return tx;
  };

  const onSubmit = async () => {
    const { nodeData } = state;
    let tx: TransactionExecuteResult | undefined;
    // TODO show a special UI for loading state
    dispatch({ type: 'set_bond_status', payload: 'loading' });
    if ((nodeData as NodeData).nodeType === 'mixnode') {
      tx = await bondMixnode();
    } else {
      tx = await bondGateway();
    }
    dispatch({ type: 'set_tx', payload: tx });
    if (state.bondStatus === 'success') {
      dispatch({ type: 'next_step' });
    }
  };

  const onConfirm = () => {
    dispatch({ type: 'close_modal' });
    dispatch({ type: 'reset' });
  };

  return (
    <NymCard title="Bonding">
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          pt: 0,
        }}
      >
        <Typography>Bond a node or a gateway</Typography>
        <Button
          disabled={false}
          variant="contained"
          color="primary"
          type="button"
          disableElevation
          onClick={() => dispatch({ type: 'show_modal' })}
          sx={{ py: 1.5, px: 3 }}
        >
          Bond
        </Button>
      </Box>
      {formStep === 1 && showModal && (
        <NodeIdentityModal
          open={formStep === 1 && showModal}
          onClose={() => dispatch({ type: 'reset' })}
          onSubmit={async (data) => {
            dispatch({ type: 'set_node_data', payload: data });
            dispatch({ type: 'next_step' });
          }}
        />
      )}
      {formStep === 2 && showModal && (
        <AmountModal
          open={formStep === 2 && showModal}
          onClose={() => dispatch({ type: 'reset' })}
          onSubmit={async (data) => {
            dispatch({ type: 'set_amount_data', payload: data });
            dispatch({ type: 'next_step' });
          }}
          nodeType={state.nodeData?.nodeType || 'mixnode'}
        />
      )}
      {formStep === 3 && showModal && (
        <SummaryModal
          open={formStep === 3 && showModal}
          onClose={() => dispatch({ type: 'reset' })}
          onCancel={() => dispatch({ type: 'prev_step' })}
          onSubmit={onSubmit}
          node={state.nodeData as NodeData}
          amount={state.amountData as MixnodeAmount | GatewayAmount}
          onError={(msg: string) => {
            dispatch({ type: 'set_error', payload: msg });
          }}
        />
      )}
      {state.bondStatus === 'success' && formStep === 4 && showModal && (
        <ConfirmationModal
          open={formStep === 4 && showModal}
          onConfirm={onConfirm}
          onClose={onConfirm}
          title="Bonding successful"
          confirmButton="Done"
          maxWidth="xs"
          fullWidth
        >
          <Link href={`${urls(network).blockExplorer}/transaction/${state.tx?.transaction_hash}`} noIcon>
            View on blockchain
          </Link>
        </ConfirmationModal>
      )}
      {state.bondStatus === 'error' && (
        <ConfirmationModal
          open={showModal}
          onClose={() => dispatch({ type: 'reset' })}
          onConfirm={() => dispatch({ type: 'reset' })}
          title="Unbonding failed"
          confirmButton="Done"
          maxWidth="xs"
        >
          <Typography variant="caption">Error: {state.error}</Typography>
          <ErrorOutline color="error" />
        </ConfirmationModal>
      )}
    </NymCard>
  );
};

export default BondingCard;
