import React, { useContext } from 'react';
import { Box, CircularProgress, Typography } from '@mui/material';
import { TransactionDetails as TTransactionDetails } from '@nymproject/types';
import { Link } from '@nymproject/react/link/Link';
import { SendError } from './SendError';
import { AppContext, urls } from '../../context/main';
import { SuccessReponse } from '../../components';
import { TransactionDetails } from '../../components/TransactionDetails';

export const SendConfirmation = ({
  data,
  error,
  isLoading,
}: {
  data?: TTransactionDetails & { tx_hash: string };
  error?: string;
  isLoading: boolean;
}) => {
  const { userBalance, clientDetails, network } = useContext(AppContext);

  if (!data && !error && !isLoading) return null;

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        width: '100%',
      }}
    >
      {isLoading && <CircularProgress size={48} />}
      {!isLoading && !!error && <SendError message={error} />}
      {!isLoading && data && (
        <>
          <SuccessReponse
            title="Transaction Complete"
            subtitle={
              <>
                Check the transaction hash{' '}
                <Link
                  href={`${urls(network).blockExplorer}/transactions/${data.tx_hash}`}
                  target="_blank"
                  text="here"
                />
              </>
            }
            caption={
              userBalance.balance && (
                <Typography>Your current balance is: {userBalance.balance.printable_balance}</Typography>
              )
            }
          />
          <TransactionDetails
            details={[
              { primary: 'Recipient', secondary: data.to_address },
              { primary: 'Amount', secondary: `${data.amount.amount} ${clientDetails?.denom}` },
            ]}
          />
        </>
      )}
    </Box>
  );
};
