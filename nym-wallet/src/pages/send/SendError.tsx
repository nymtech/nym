import React from 'react'
import { Alert, Box, Card, Typography } from '@mui/material'
import { ErrorOutline } from '@mui/icons-material'

export const SendError = ({ message }: { message?: string }) => {
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
          <ErrorOutline sx={{ fontSize: 50, color: 'error.main' }} />
          <Typography>Transaction failed</Typography>
        </Box>

        <Card variant="outlined" sx={{ width: '100%', p: 2 }}>
          <Alert severity="error" data-testid="transaction-error">
            An error occured during the request {message}
          </Alert>
        </Card>
      </>
    </Box>
  )
}
