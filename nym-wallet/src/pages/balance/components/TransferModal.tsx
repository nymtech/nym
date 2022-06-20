import React, { useContext, useEffect, useState } from 'react';
import { Box, Card, CardContent, CircularProgress, Stack, Typography } from '@mui/material';
import { ArrowForwardSharp, Check, WarningOutlined } from '@mui/icons-material';
import { FeeDetails } from '@nymproject/types';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { ModalListItem } from 'src/components/Delegation/ModalListItem';
import { ModalDivider } from 'src/components/Modals/ModalDivider';
import { AppContext } from 'src/context';
import { withdrawVestedCoins } from 'src/requests';
import { Console } from 'src/utils/console';
import { simulateWithdrawVestedCoins } from 'src/requests/simulate';

type TResponseState = 'loading' | 'success' | 'fail';

export const TransferModal = ({ onClose }: { onClose: () => void }) => {
  const [state, setState] = useState<TResponseState>();
  const [fee, setFee] = useState<FeeDetails>();

  const { userBalance, clientDetails } = useContext(AppContext);

  const getStateIcon = (reqState?: TResponseState) => {
    switch (reqState) {
      case 'loading':
        return <CircularProgress />;
      case 'success':
        return <Check color="success" />;
      case 'fail':
        return <WarningOutlined color="error" />;
      default:
        return <ArrowForwardSharp fontSize="large" />;
    }
  };

  const getFee = async () => {
    if (userBalance.tokenAllocation?.spendable && clientDetails?.denom) {
      try {
        const simulatedFee = await simulateWithdrawVestedCoins({
          amount: { amount: userBalance.tokenAllocation?.spendable, denom: clientDetails?.denom },
        });
        setFee(simulatedFee);
      } catch (e) {
        setFee({ amount: { amount: 'n/a', denom: clientDetails.denom } });
        Console.error(e);
      }
    }
  };

  useEffect(() => {
    getFee();
  }, []);

  const handleTransfer = async () => {
    if (userBalance.tokenAllocation?.spendable && clientDetails?.denom) {
      setState('loading');
      try {
        await withdrawVestedCoins({
          amount: userBalance.tokenAllocation?.spendable,
          denom: clientDetails.denom,
        });
        setState('success');
        await userBalance.refreshBalances();
      } catch (e) {
        Console.error(e as string);
        setState('fail');
      }
    }
  };

  return (
    <SimpleModal
      open
      okLabel={state === 'loading' ? 'Transferring' : 'Transfer'}
      header="Transfer locked tokens"
      subHeader="Transfer locked tokens to balance"
      sx={{ width: 600 }}
      onOk={handleTransfer}
      okDisabled={state === 'loading' || !fee || userBalance.tokenAllocation?.spendable === '0'}
      onClose={onClose}
    >
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        <Card elevation={0} sx={{ minWidth: 225, bgcolor: 'grey.100' }}>
          <CardContent>
            <Typography variant="caption" sx={{ color: 'grey.600' }}>
              Locked balance
            </Typography>
            <Typography variant="h5" sx={{ fontWeight: 700 }}>
              {userBalance.tokenAllocation?.spendable} {clientDetails?.denom}
            </Typography>
          </CardContent>
        </Card>
        {getStateIcon(state)}
        <Card elevation={0} sx={{ minWidth: 225, bgcolor: 'grey.100' }}>
          <CardContent>
            <Typography variant="caption" sx={{ color: 'grey.600' }}>
              Liquid balance
            </Typography>
            <Typography variant="h5" sx={{ fontWeight: 700 }}>
              {userBalance.balance?.printable_balance}
            </Typography>
          </CardContent>
        </Card>
      </Stack>
      <Box sx={{ mt: 3 }}>
        <ModalDivider />
        <ModalListItem
          label="Est. fee for this transaction"
          value={fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : <CircularProgress size={15} />}
          divider
        />
      </Box>
    </SimpleModal>
  );
};
