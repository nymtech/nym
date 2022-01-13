import React from 'react'
import { Divider, Stack, Typography } from '@mui/material'
import { TMixnodeBondDetails } from '../../types'

export const Overview = ({ details }: { details?: TMixnodeBondDetails | null }) => (
  <Stack spacing={3} sx={{ p: 4, pb: 0 }}>
    <Typography sx={{ color: 'grey.600' }}>Node identity:  {details?.mix_node.identity_key || 'n/a'}</Typography>
    <Divider />
  </Stack>
)
