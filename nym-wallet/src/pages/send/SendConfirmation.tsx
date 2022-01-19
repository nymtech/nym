import React, { useContext } from 'react'
import { Box, CircularProgress, Link, Typography } from '@mui/material'
import { SendError } from './SendError'
import { ClientContext, MAJOR_CURRENCY, urls } from '../../context/main'
import { SuccessReponse } from '../../components'
import { TransactionDetails } from '../../components/TransactionDetails'
import { TransactionDetails as TTransactionDetails } from '../../types'

export const SendConfirmation = ({
  data,
  error,
  isLoading,
}: {
  data?: TTransactionDetails & { tx_hash: string }
  error?: string
  isLoading: boolean
}) => {
  const { userBalance, clientDetails } = useContext(ClientContext)
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
                <Link href={`${urls.blockExplorer}/transactions/${data.tx_hash}`} target="_blank">
                  here
                </Link>
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
              { primary: 'Amount', secondary: `${data.amount.amount} ${MAJOR_CURRENCY}` },
            ]}
          />
        </>
      )}
    </Box>
  )
}
