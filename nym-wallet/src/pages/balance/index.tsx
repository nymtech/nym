import React, { useContext, useEffect } from 'react'
import { Box } from '@mui/material'
import { BalanceCard } from './balance'
import { VestingCard } from './vesting'
import { ClientContext, urls } from '../../context/main'
import { Layout } from '../../layouts'

export const Balance = () => {
  const { userBalance } = useContext(ClientContext)

  useEffect(() => {
    userBalance.fetchBalance()
  }, [])

  return (
    <Layout>
      <Box display="flex" flexDirection="column" gap={2}>
        <BalanceCard />
        <VestingCard />
      </Box>
    </Layout>
  )
}
