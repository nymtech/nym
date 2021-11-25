import React, { useContext, useEffect, useState } from 'react'
import {
  Alert,
  Box,
  CardContent,
  CircularProgress,
  IconButton,
  Tooltip,
  Typography,
} from '@mui/material'
import { ClientContext } from '../context/main'
import { CheckCircleOutline, FileCopy, Refresh } from '@mui/icons-material'
import { NymCard } from './NymCard'
import { handleCopy } from './CopyToClipboard'
import { truncate } from '../utils'

export const BalanceCard = () => {
  const { getBalance } = useContext(ClientContext)

  useEffect(getBalance.fetchBalance, [])

  return (
    <Box sx={{ margin: 3 }}>
      <NymCard
        title="Balance"
        subheader="Current wallet balance"
        noPadding
        Action={
          <Tooltip title="Refresh balance">
            <IconButton
              data-testid="refresh-balance"
              onClick={getBalance.fetchBalance}
              size="small"
            >
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
    </Box>
  )
}
enum EnumCopyState {
  copying,
  copySuccess,
}

export const AddressCard = () => {
  const { clientDetails } = useContext(ClientContext)

  const [copyState, setCopyState] = useState<EnumCopyState>()

  return (
    <Box sx={{ margin: 3 }}>
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
                  <CheckCircleOutline sx={{ color: 'palette.success.main' }} />
                ) : (
                  <FileCopy />
                )}
              </IconButton>
            </span>
          </Tooltip>
        }
      >
        <CardContent>
          <Typography
            data-testid="wallet-address"
            style={{ fontWeight: 'regular' }}
          >
            {truncate(clientDetails?.client_address!, 35)}
          </Typography>
        </CardContent>
      </NymCard>
    </Box>
  )
}
