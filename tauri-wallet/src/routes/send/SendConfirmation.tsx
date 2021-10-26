import React from 'react'
import { Card, CircularProgress, Theme, Typography } from '@material-ui/core'
import { CheckCircleOutline } from '@material-ui/icons'
import { useTheme } from '@material-ui/styles'
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
  const theme: Theme = useTheme()

  return (
    <div
      style={{
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
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              marginBottom: theme.spacing(4),
            }}
          >
            <CheckCircleOutline
              style={{
                fontSize: 50,
                color: theme.palette.success.main,
                marginBottom: theme.spacing(1),
              }}
            />
            <Typography data-testid="transaction-complete">Transaction complete</Typography>
          </div>

          <Card
            variant="outlined"
            style={{ width: '100%', padding: theme.spacing(2) }}
          >
            <div style={{ display: 'flex', marginBottom: theme.spacing(2) }}>
              <div style={{ width: '33%' }}>
                <Typography style={{ color: theme.palette.grey[600] }}>
                  Recipient
                </Typography>
              </div>
              <div style={{ wordBreak: 'break-all' }}>
                <Typography data-testid="to-address">{data.to_address}</Typography>
              </div>
            </div>
            <div style={{ display: 'flex' }}>
              <div style={{ width: '33%' }}>
                <Typography style={{ color: theme.palette.grey[600] }}>
                  Amount
                </Typography>
              </div>
              <div>
                <Typography data-testid="send-amount">{data.amount.amount + ' punks'}</Typography>
              </div>
            </div>
          </Card>
        </>
      )}
    </div>
  )
}
