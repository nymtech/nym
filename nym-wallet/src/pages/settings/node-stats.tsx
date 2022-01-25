import { OpenInNew } from '@mui/icons-material'
import { Button, Link, Stack, Typography } from '@mui/material'
import React, { useEffect } from 'react'
import { urls } from '../../context/main'

export const NodeStats = ({ mixnodeId }: { mixnodeId?: string }) => {
  return (
    <Stack spacing={2} sx={{ p: 4 }}>
      <Typography>All your node stats are available on the link below</Typography>
      <Link href={`${urls.networkExplorer}/network-components/mixnodes/${mixnodeId}`} target="_blank">
        <Button endIcon={<OpenInNew />}>Network Explorer</Button>
      </Link>
    </Stack>
  )
}
