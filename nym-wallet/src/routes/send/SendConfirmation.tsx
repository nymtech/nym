import React, { useContext } from 'react'
import { Grid, Stack, Box, Card, CircularProgress, Link, Typography } from '@mui/material'
import { SendError } from './SendError'
import { TauriTxResult } from '../../types/rust/tauritxresult'
import { ClientContext, urls } from '../../context/main'

export const SendConfirmation = ({
  data,
  error,
  isLoading,
}: {
  data?: TauriTxResult['details']
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
          <Stack spacing={3} alignItems="center" sx={{ mb: 5 }}>
            <Typography variant="h5" fontWeight="600" data-testid="transaction-complete" color="success.main">
              Transaction Complete
            </Typography>
            <Typography>
              Check the transaction hash{' '}
              <Link href={`${urls.blockExplorer}/account/${clientDetails?.client_address}`} target="_blank">
                here
              </Link>
            </Typography>
            {userBalance.balance && (
              <Typography>Your current balance is: {userBalance.balance.printable_balance}</Typography>
            )}
          </Stack>

          <Card variant="outlined" sx={{ width: '100%', p: 2 }}>
            <Grid container>
              <Grid item sm={4} md={3} lg={2}>
                <Typography sx={{ color: (theme) => theme.palette.grey[600] }}>Recipient</Typography>
              </Grid>
              <Grid item>
                <Typography data-testid="to-address">{data.to_address}</Typography>
              </Grid>
            </Grid>
            <Grid container>
              <Grid item sm={4} md={3} lg={2}>
                <Typography sx={{ color: (theme) => theme.palette.grey[600] }}>Amount</Typography>
              </Grid>
              <Grid item>
                <Typography data-testid="send-amount">{data.amount.amount + ' punks'}</Typography>
              </Grid>
            </Grid>
          </Card>
        </>
      )}
    </Box>
  )
}
