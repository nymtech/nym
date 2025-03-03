import React from 'react'
import { Box, Typography } from '@mui/material'
import { useWalletContext } from '@/app/context/wallet'
import { useIsMobile } from '@/app/hooks'
import { TokenSVG } from '@/app/icons/TokenSVG'

export const WalletBalance = () => {
  const { balance } = useWalletContext()
  const isMobile = useIsMobile(1200)

  const showBalance = !isMobile && balance.status === 'success'

  if (!showBalance) {
    return null
  }

  return (
    <Box display="flex" alignItems="center" gap={1}>
      <TokenSVG />
      <Typography variant="body1" fontWeight={600}>
        {balance.data} NYM
      </Typography>
    </Box>
  )
}
