import React, { useContext } from 'react'
import { Box, Grid, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { MainCard, TopCard } from './Cards'
import { InboundCard, LimitCard, OutboundCard } from './DataCards'
import { Info } from './Info'

type TDashboardProps = {
  plan: string
  buyBandwidth: () => void
}

export const Dashboard: React.FC<TDashboardProps> = ({
  plan,
  buyBandwidth,
}) => {
  const { ss5IsActive, toggleSs5, bandwidthLimit, bandwidthUsed } =
    useContext(ClientContext)
  const theme: Theme = useTheme()
  return (
    <NymCard
      title="SOCKS5 Dashboard"
      subheader="Monitor your SOCKS5 usage"
      Action={<Info />}
    >
      <Box padding={theme.spacing(0.5)}>
        <Grid container spacing={6}>
          <Grid item xs={12}>
            <TopCard
              isActive={ss5IsActive}
              toggleIsActive={toggleSs5}
              plan={plan}
              disabled={bandwidthLimit === bandwidthUsed}
            />
          </Grid>
          <Grid item xs={12}>
            <MainCard
              isActive={ss5IsActive}
              toggleIsActive={toggleSs5}
              disabled={bandwidthLimit === bandwidthUsed}
              buyBandwidth={buyBandwidth}
            />
          </Grid>

          <Grid item xs={4}>
            <OutboundCard isActive={ss5IsActive} />
          </Grid>
          <Grid item xs={4}>
            <InboundCard isActive={ss5IsActive} />
          </Grid>
          <Grid item xs={4}>
            <LimitCard isActive={ss5IsActive} />
          </Grid>
        </Grid>
      </Box>
    </NymCard>
  )
}
