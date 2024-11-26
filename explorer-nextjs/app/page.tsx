'use client'

import React, { useEffect } from 'react'
import { Box, Grid, Link, Typography } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import { PeopleAlt } from '@mui/icons-material'
import { Title } from '@/app/components/Title'
import { StatsCard } from '@/app/components/StatsCard'
import { MixnodesSVG } from '@/app/icons/MixnodesSVG'
import { Icons } from '@/app/components/Icons'
import { GatewaysSVG } from '@/app/icons/GatewaysSVG'
import { ValidatorsSVG } from '@/app/icons/ValidatorsSVG'
import { ContentCard } from '@/app/components/ContentCard'
import { WorldMap } from '@/app/components/WorldMap'
import { BLOCK_EXPLORER_BASE_URL } from '@/app/api/constants'
import { formatNumber } from '@/app/utils'
import { useMainContext } from './context/main'
import { useRouter } from 'next/navigation'

const PageOverview = () => {
  const theme = useTheme()
  const router = useRouter()

  const {
    summaryOverview,
    gateways,
    validators,
    block,
    countryData,
    serviceProviders,
  } = useMainContext()
  return (
    <Box component="main" sx={{ flexGrow: 1 }}>
      <Grid>
        <Grid item paddingBottom={3}>
          <Title text="Overview" />
        </Grid>
        <Grid item>
          <Grid container spacing={3}>
            {summaryOverview && (
              <>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => router.push('/network-components/nodes')}
                    title="Nodes"
                    icon={<MixnodesSVG />}
                    count={summaryOverview.data?.mixnodes.count || ''}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
              </>
            )}
            {summaryOverview && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => router.push('/network-components/gateways')}
                  title="Entry Gateways"
                  count={summaryOverview.data?.nymnodes.roles.entry || ''}
                  icon={<GatewaysSVG />}
                />
              </Grid>
            )}
            {summaryOverview && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => router.push('/network-components/gateways')}
                  title="Exit Gateways"
                  count={summaryOverview.data?.nymnodes.roles.exit_ipr || ''}
                  icon={<GatewaysSVG />}
                />
              </Grid>
            )}
            {summaryOverview && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => router.push('/network-components/gateways')}
                  title="SOCKS5 Network Requesters"
                  count={summaryOverview.data?.nymnodes.roles.exit_nr || ''}
                  icon={<GatewaysSVG />}
                />
              </Grid>
            )}
            {validators && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => window.open(`${BLOCK_EXPLORER_BASE_URL}/validators`)}
                  title="Validators"
                  count={validators?.data?.count || ''}
                  errorMsg={validators?.error}
                  icon={<ValidatorsSVG />}
                />
              </Grid>
            )}
            {block?.data && (
              <Grid item xs={12}>
                <Link
                  href={`${BLOCK_EXPLORER_BASE_URL}/blocks`}
                  target="_blank"
                  rel="noreferrer"
                  underline="none"
                  color="inherit"
                  marginY={2}
                  paddingX={3}
                  paddingY={0.25}
                  fontSize={14}
                  fontWeight={600}
                  display="flex"
                  alignItems="center"
                >
                  <Typography fontWeight="inherit" fontSize="inherit">
                    Current block height is {formatNumber(block.data)}
                  </Typography>
                  <OpenInNewIcon
                    fontWeight="inherit"
                    fontSize="inherit"
                    sx={{ ml: 0.5 }}
                  />
                </Link>
              </Grid>
            )}
            <Grid item xs={12}>
              <ContentCard title="Distribution of nodes around the world">
                <WorldMap loading={false} countryData={countryData} />
              </ContentCard>
            </Grid>
          </Grid>
        </Grid>
      </Grid>
    </Box>
  )
}

export default PageOverview
