import React, { useContext } from 'react'
import {
  CardContent,
  IconButton,
  Typography,
  useTheme,
} from '@material-ui/core'
import { ClientContext } from '../context/main'
import { FileCopy, Refresh } from '@material-ui/icons'
import { NymCard } from './NymCard'

export const BalanceCard = () => {
  const theme = useTheme()
  const { client } = useContext(ClientContext)

  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Balance"
        subheader="Current wallet balance"
        noPadding
        Action={
          <IconButton>
            <Refresh />
          </IconButton>
        }
      >
        <CardContent>
          <Typography>{client.balance}</Typography>
        </CardContent>
      </NymCard>
    </div>
  )
}

export const AddressCard = () => {
  const theme = useTheme()
  const { client } = useContext(ClientContext)
  return (
    <div style={{ margin: theme.spacing(3) }}>
      <NymCard
        title="Address"
        subheader="Wallet payments address"
        noPadding
        Action={
          <IconButton>
            <FileCopy />
          </IconButton>
        }
      >
        <CardContent>{client.address}</CardContent>
      </NymCard>
    </div>
  )
}
