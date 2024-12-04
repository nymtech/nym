import React from 'react'
import { LinearProgress, Box } from '@mui/material'

export default function Loading() {
  return (
    <Box sx={{ py: 16 }}>
      <LinearProgress />
    </Box>
  )
}
