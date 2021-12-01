import React from 'react'
import { Box, Card, CircularProgress, Typography } from '@mui/material'
import { CheckCircleOutline } from '@mui/icons-material'
import { SendError } from './SendError'
import { TauriTxResult } from '../../types/rust/tauritxresult'

export const SendConfirmation = ({
  data,
  error,
  isLoading,
}: {
  data?: TauriTxResult['details']
  error?: string
  isLoading: boolean
}) => {
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
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              mb: 4,
            }}
          >
            <CheckCircleOutline
              sx={{
                fontSize: 50,
                color: 'success.main',
                mb: 1,
              }}
            />
            <Typography data-testid="transaction-complete">
              Transaction complete
            </Typography>
          </Box>

          <Card variant="outlined" sx={{ width: '100%', p: 2 }}>
            <Box sx={{ display: 'flex', mb: 2 }}>
              <Box sx={{ width: '1/3' }}>
                <Typography sx={{ color: (theme) => theme.palette.grey[600] }}>
                  Recipient
                </Typography>
              </Box>
              <Box sx={{ wordBreak: 'break-all' }}>
                <Typography data-testid="to-address">
                  {data.to_address}
                </Typography>
              </Box>
            </Box>
            <Box sx={{ display: 'flex' }}>
              <Box sx={{ width: '33%' }}>
                <Typography sx={{ color: (theme) => theme.palette.grey[600] }}>
                  Amount
                </Typography>
              </Box>
              <Box>
                <Typography data-testid="send-amount">
                  {data.amount.amount + ' punks'}
                </Typography>
              </Box>
            </Box>
          </Card>
        </>
      )}
    </Box>
  )
}
