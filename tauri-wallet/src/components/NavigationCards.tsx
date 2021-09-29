import React, { useContext, useEffect, useState } from 'react'
import {
  Box,
  CardContent,
  CircularProgress,
  IconButton,
  Theme,
  Tooltip,
  Typography,
  useTheme,
} from '@material-ui/core'
import { ClientContext } from '../context/main'
import {
  ArrowForwardSharp,
  CheckCircleOutline,
  FileCopy,
  PowerSettingsNew,
  Refresh,
} from '@material-ui/icons'
import { NymCard } from './NymCard'
import { Alert } from '@material-ui/lab'
import { handleCopy } from './CopyToClipboard'
import { truncate } from '../utils'
import { useHistory } from 'react-router'

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
            <IconButton onClick={getBalance.fetchBalance} size="small">
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
              <Typography variant="h6">
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
          <Typography
            style={{ fontWeight: theme.typography.fontWeightRegular }}
          >
            {truncate(clientDetails?.client_address!, 35)}
          </Typography>
        </CardContent>
      </NymCard>
    </div>
  )
}

export const SockS5 = () => {
  const theme: Theme = useTheme()
  const history = useHistory()
  const { ss5IsActive, bandwidthLimit, toggleSs5 } = useContext(ClientContext)

  if (bandwidthLimit === 0) return null

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Socks5"
        Icon={
          <IconButton onClick={toggleSs5}>
            <PowerSettingsNew
              style={{
                color: ss5IsActive
                  ? theme.palette.success.main
                  : theme.palette.error.main,
              }}
            />
          </IconButton>
        }
        Action={
          <Box style={{ marginTop: theme.spacing(1) }}>
            <IconButton onClick={() => history.push('/socks5')}>
              <ArrowForwardSharp />
            </IconButton>
          </Box>
        }
      />
    </div>
  )
}
