import React, { useContext } from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { TransactionExecuteResult } from '@nymproject/types';
import { SuccessReponse, TransactionDetails } from '../../components';
import { AppContext } from '../../context/main';

export const SuccessView: React.FC<{ details?: { amount: string; result: TransactionExecuteResult } }> = ({
  details,
}) => {
  const { userBalance, clientDetails } = useContext(AppContext);
  return (
    <>
      <SuccessReponse
        title="Delegation Request Complete"
        subtitle={
          <Stack alignItems="center" spacing={1}>
            <Typography>Successfully requested delegation to node </Typography>
            <Typography sx={{ textDecoration: 'underline', fontWeight: 600 }}>
              Note it may take up to one hour to take effect
            </Typography>
          </Stack>
        }
        caption={`Your current balance is: ${userBalance.balance?.printable_balance}`}
      />
      {details && (
        <Box sx={{ mt: 2 }}>
          <TransactionDetails
            details={[
              { primary: 'Node', secondary: details.result.transaction_hash },
              { primary: 'Amount', secondary: `${details.amount} ${clientDetails?.denom}` },
            ]}
          />
        </Box>
      )}
    </>
  );
};
