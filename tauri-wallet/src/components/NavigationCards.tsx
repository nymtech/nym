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

export const BalanceCard = () => {
  const theme = useTheme()
  const { balance, balanceError, balanceLoading, getBalance } =
    useContext(ClientContext)

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Balance"
        subheader="Current wallet balance"
        noPadding
        Action={
          <Tooltip title="Refresh balance">
            <IconButton onClick={getBalance}>
              <Refresh />
            </IconButton>
          </Tooltip>
        }
      >
        <CardContent>
          <div style={{ display: 'flex', justifyContent: 'center' }}>
            {balanceLoading ? (
              <CircularProgress size={24} />
            ) : balanceError ? (
              <Alert severity="error" style={{ width: '100%' }}>
                {balanceError}
              </Alert>
            ) : (
              <Typography>{balance?.printableBalance}</Typography>
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
        <CardContent>{clientDetails?.client_address}</CardContent>
      </NymCard>
    </div>
  )
}
