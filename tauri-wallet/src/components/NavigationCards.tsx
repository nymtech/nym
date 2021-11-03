import React, { useContext, useEffect, useState } from 'react'
import {
  CardContent,
  CircularProgress,
  IconButton,
  Tooltip,
  Typography,
  useTheme,
} from '@material-ui/core'
import { ClientContext } from '../context/main'
import { CheckCircleOutline, FileCopy, Refresh } from '@material-ui/icons'
import { NymCard } from './NymCard'
import { Alert } from '@material-ui/lab'
import { handleCopy } from './CopyToClipboard'
import { truncate } from '../utils'

export const BalanceCard = () => {
  const { getBalance } = useContext(ClientContext)
  const theme = useTheme()

  useEffect(getBalance.fetchBalance, [])

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Balance"
        subheader="Current wallet balance"
        noPadding
        Action={
          <Tooltip title="Refresh balance">
            <IconButton data-testid="refresh-balance" onClick={getBalance.fetchBalance} size="small">
              <Refresh />
            </IconButton>
          </Tooltip>
        }
      >
        <CardContent>
          <div style={{ display: 'flex', justifyContent: 'center' }}>
            {getBalance.isLoading ? (
              <CircularProgress size={24} />
            ) : getBalance.error ? (
              <Alert severity="error" style={{ width: '100%' }}>
                {getBalance.error}
              </Alert>
            ) : (
              <Typography variant="h6" data-testid="account-balance">
                {getBalance.balance?.printable_balance}
              </Typography>
            )}
          </div>
        </CardContent>
      </NymCard>
    </div>
  )
}
enum EnumCopyState {
  copying,
  copySuccess,
}

export const AddressCard = () => {
  const { clientDetails } = useContext(ClientContext)

  const [copyState, setCopyState] = useState<EnumCopyState>()

  const theme = useTheme()

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Address"
        subheader="Wallet payments address"
        noPadding
        data-testid="wallet-address-header"
        Action={
          <Tooltip title={!copyState ? 'Copy address' : 'Copied'}>
            <span>
              <IconButton
                disabled={!!copyState}
                onClick={async () => {
                  setCopyState(EnumCopyState.copying)
                  await handleCopy({
                    text: clientDetails?.client_address || '',
                    cb: (isCopied) => {
                      if (isCopied) {
                        setCopyState(EnumCopyState.copySuccess)
                        setTimeout(() => {
                          setCopyState(undefined)
                        }, 2500)
                      }
                    },
                  })
                }}
              >
                {copyState === EnumCopyState.copying ? (
                  <CircularProgress size={24} />
                ) : copyState === EnumCopyState.copySuccess ? (
                  <CheckCircleOutline
                    style={{ color: theme.palette.success.main }}
                  />
                ) : (
                  <FileCopy />
                )}
              </IconButton>
            </span>
          </Tooltip>
        }
      >
        <CardContent>
          <Typography data-testid="wallet-address"
            style={{ fontWeight: theme.typography.fontWeightRegular }}
          >
            {truncate(clientDetails?.client_address!, 35)}
          </Typography>
        </CardContent>
      </NymCard>
    </div>
  )
}
