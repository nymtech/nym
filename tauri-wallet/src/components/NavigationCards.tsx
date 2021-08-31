import React, { useContext, useEffect } from 'react'
import {
  CardContent,
  CircularProgress,
  IconButton,
  Typography,
  useTheme,
} from '@material-ui/core'
import { ClientContext } from '../context/main'
import { FileCopy, Refresh } from '@material-ui/icons'
import { NymCard } from './NymCard'
import { Alert } from '@material-ui/lab'
import { handleCopy } from './CopyToClipboard'

export const BalanceCard = () => {
  const theme = useTheme()
  const { balance, balanceError, balanceLoading, getBalance } =
    useContext(ClientContext)

  useEffect(() => {
    getBalance()
  }, [])

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Balance"
        subheader="Current wallet balance"
        noPadding
        Action={
          <IconButton onClick={getBalance}>
            <Refresh />
          </IconButton>
        }
      >
        <CardContent>
          <div style={{ display: 'flex', justifyContent: 'center' }}>
            {balanceLoading ? (
              <CircularProgress size={28} />
            ) : balanceError ? (
              <Alert severity="error" style={{ width: '100%' }}>
                {balanceError}
              </Alert>
            ) : (
              <Typography>{balance?.amount}</Typography>
            )}
          </div>
        </CardContent>
      </NymCard>
    </div>
  )
}

export const AddressCard = () => {
  const theme = useTheme()
  const { clientDetails } = useContext(ClientContext)
  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Address"
        subheader="Wallet payments address"
        noPadding
        Action={
          <IconButton
            onClick={() =>
              handleCopy({
                text: clientDetails?.client_address || '',
                cb: () => {},
              })
            }
          >
            <FileCopy />
          </IconButton>
        }
      >
        <CardContent>{clientDetails?.client_address}</CardContent>
      </NymCard>
    </div>
  )
}
