import React, { useEffect, useState } from 'react'
import { Card, CircularProgress, Theme, Typography } from '@material-ui/core'
import { CheckCircleOutline } from '@material-ui/icons'
import { useTheme } from '@material-ui/styles'

export const SendConfirmation = ({
  amount,
  recipient,
  onFinish,
}: {
  amount: string
  recipient: string
  onFinish: () => void
}) => {
  const theme: Theme = useTheme()
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    setTimeout(() => {
      setIsLoading(false)
      onFinish()
    }, 3000)
  }, [])

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
      {isLoading ? (
        <CircularProgress size={48} />
      ) : (
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
            <Typography>Transaction complete</Typography>
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
                <Typography>{recipient}</Typography>
              </div>
            </div>
            <div style={{ display: 'flex' }}>
              <div style={{ width: '33%' }}>
                <Typography style={{ color: theme.palette.grey[600] }}>
                  Amount
                </Typography>
              </div>
              <div>
                <Typography>{amount + ' punks'}</Typography>
              </div>
            </div>
          </Card>
        </>
      )}
    </div>
  )
}
