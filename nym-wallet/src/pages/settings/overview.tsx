import React from 'react'
import { Divider, Stack, Typography } from '@mui/material'
import { CheckCircleOutline } from '@mui/icons-material'

export const Overview = () => (
  <Stack spacing={2}>
    <Typography sx={{ color: 'grey.600' }}>Node identity 94oh6aU4myLjDusK6QeTWEPUc3nm4vYPCsKkdcjYhRLd</Typography>
    <Typography sx={{ color: 'success.main', display: 'flex', alignItems: 'center' }}>
      <CheckCircleOutline fontSize="small" color="success" sx={{ mr: 1 }} /> Mixnode is Active this epoch
    </Typography>
    <Divider />
  </Stack>
)
