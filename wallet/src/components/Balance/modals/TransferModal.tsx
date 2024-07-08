import React, { useContext, useEffect, useState } from 'react';
import { Alert, Box, CircularProgress } from '@mui/material';
import { SimpleModal } from '@src/components/Modals/SimpleModal';
import { ModalListItem } from '@src/components/Modals/ModalListItem';
import { AppContext, urls } from '@src/context';
import { FeeWarning } from '@src/components/FeeWarning';
import { withdrawVestedCoins } from '@src/requests';
import { Console } from '@src/utils/console';
import { simulateWithdrawVestedCoins } from '@src/requests/simulate';
import { useGetFee } from '@src/hooks/useGetFee';
import { SuccessModal, TTransactionDetails } from './TransferModalSuccess';
import { TResponseState } from '../../../pages/balance/types';

export const TransferModal = ({ onClose }: { onClose: () => void }) => {
  const [state, setState] = useState<TResponseState>();

  const [tx, setTx] = useState<TTransactionDetails>();

  const { userBalance, clientDetails, network } = useContext(AppContext);
  const { fee, getFee } = useGetFee();

  useEffect(() => {
    getFee(simulateWithdrawVestedCoins, {
      amount: { amount: userBalance.tokenAllocation?.spendable, denom: clientDetails?.display_mix_denom },
    });
  }, []);

  const handleTransfer = async () => {
    if (userBalance.tokenAllocation?.spendable && clientDetails?.display_mix_denom) {
      setState('loading');
      try {
        const txResponse = await withdrawVestedCoins(
          {
            amount: userBalance.tokenAllocation?.spendable,
            denom: clientDetails?.display_mix_denom,
          },
          fee?.fee,
        );
        setState('success');
        setTx({
          amount: `${userBalance.tokenAllocation?.spendable} ${clientDetails.display_mix_denom.toUpperCase()}`,
          url: `${urls(network).blockExplorer}/transaction/${txResponse.transaction_hash}`,
        });
        await userBalance.refreshBalances();
      } catch (e) {
        Console.error(e as string);
        setState('fail');
      }
    }
  };

  if (state === 'success') {
    return <SuccessModal onClose={onClose} tx={tx} />;
  }

  return (
    <SimpleModal
      open
      okLabel={state === 'loading' ? 'Transferring..' : 'Transfer'}
      header="Transfer locked tokens"
      subHeader="Transfer locked tokens to balance"
      sx={{ width: 600 }}
      onOk={handleTransfer}
      okDisabled={state === 'loading' || !fee || userBalance.tokenAllocation?.spendable === '0'}
      onClose={onClose}
    >
      <Box sx={{ mt: 3 }}>
        {state === 'loading' ? (
          <Box sx={{ display: 'flex', justifyContent: 'center' }}>
            <CircularProgress />
          </Box>
        ) : (
          <>
            <ModalListItem
              label="Unlocked transferrable tokens"
              value={`${userBalance.tokenAllocation?.spendable} ${clientDetails?.display_mix_denom.toUpperCase()}`}
              divider
            />
            <ModalListItem
              label="Est. fee for this transaction"
              value={fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : <CircularProgress size={15} />}
              divider
            />
            {userBalance.tokenAllocation?.spendable && fee && (
              <FeeWarning fee={fee} amount={+userBalance.tokenAllocation.spendable} />
            )}
          </>
        )}
      </Box>
      {state === 'fail' && <Alert severity="error">Transfer failed please try again in a few minutes</Alert>}
    </SimpleModal>
  );
};
