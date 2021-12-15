import React from 'react'
import { Divider, Stack, Typography } from '@mui/material'
import { CheckCircleOutline, CircleOutlined, PauseCircleOutlined } from '@mui/icons-material'

type TMixnodeStatus = 'active' | 'inactive' | 'standby'

export const Overview = ({ mixnodeStatus }: { mixnodeStatus: TMixnodeStatus }) => (
  <Stack spacing={3} sx={{ mb: 3 }}>
    <Typography sx={{ color: 'grey.600' }}>Node identity 94oh6aU4myLjDusK6QeTWEPUc3nm4vYPCsKkdcjYhRLd</Typography>
    {mixnodeStatus === 'active' && <ActiveMessage />}
    {mixnodeStatus === 'inactive' && <InActiveMessage />}
    {mixnodeStatus === 'standby' && <StandbyMessage />}
    <Divider />
  </Stack>
)

const ActiveMessage = () => (
  <Typography sx={{ color: 'success.main', display: 'flex', alignItems: 'center' }}>
    <CheckCircleOutline fontSize="small" color="success" sx={{ mr: 1 }} /> Mixnode is active in this epoch
  </Typography>
)

const InActiveMessage = () => (
  <Typography sx={{ color: 'nym.text.dark', display: 'flex', alignItems: 'center' }}>
    <CircleOutlined fontSize="small" sx={{ color: 'nym.text.dark', mr: 1 }} /> Mixnode is active in this epoch
  </Typography>
)

const StandbyMessage = () => (
  <Typography sx={{ color: 'info.main', display: 'flex', alignItems: 'center' }}>
    <PauseCircleOutlined fontSize="small" color="info" sx={{ mr: 1 }} /> Mixnode is on standy by in this epoch
  </Typography>
)
