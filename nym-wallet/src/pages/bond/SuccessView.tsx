import React, { useContext } from 'react';
import { Box } from '@mui/material';
import { SuccessReponse, TransactionDetails } from '../../components';
import { AppContext } from '../../context/main';
import { useCheckOwnership } from '../../hooks/useCheckOwnership';

export const SuccessView: React.FC<{ details?: { amount: string; address: string } }> = ({ details }) => {
  const { userBalance, currency } = useContext(AppContext);
  const { ownership } = useCheckOwnership();

  return (
    <>
      <SuccessReponse
        title="Bonding Complete"
        subtitle="Successfully bonded to node with following details"
        caption={
          ownership.vestingPledge
            ? `Your current locked balance is: ${userBalance.tokenAllocation?.locked}${currency?.major}`
            : `Your current balance is: ${userBalance.balance?.printable_balance}`
        }
      />
      {details && (
        <Box sx={{ mt: 2 }}>
          <TransactionDetails
            details={[
              { primary: 'Node', secondary: details.address },
              { primary: 'Amount', secondary: `${details.amount} ${currency?.major}` },
            ]}
          />
        </Box>
      )}
    </>
  );
};
