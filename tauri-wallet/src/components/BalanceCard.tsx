import React, { useContext } from 'react'
import { CardContent, IconButton, useTheme } from '@material-ui/core'
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
        noPadding
        Action={
          <IconButton>
            <Refresh />
          </IconButton>
        }
      >
        <CardContent>{client.balance}</CardContent>
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
