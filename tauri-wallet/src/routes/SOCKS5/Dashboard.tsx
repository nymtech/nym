import React, { useContext } from 'react'
import { Box, Grid, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { MainCard, TopCard } from './Cards'
import { DownloadCard, LimitCard, UploadCard } from './DataCards'
import { Info } from './Info'

type TDashboardProps = {
  plan: string
}

export const Dashboard: React.FC<TDashboardProps> = ({ plan }) => {
  const { ss5IsActive, toggleSs5 } = useContext(ClientContext)
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
            />
          </Grid>
          <Grid item xs={12}>
            <MainCard isActive={ss5IsActive} toggleIsActive={toggleSs5} />
          </Grid>

          <Grid item xs={4}>
            <UploadCard isActive={ss5IsActive} />
          </Grid>
          <Grid item xs={4}>
            <DownloadCard isActive={ss5IsActive} />
          </Grid>
          <Grid item xs={4}>
            <LimitCard isActive={ss5IsActive} />
          </Grid>
        </Grid>
      </Box>
    </NymCard>
  )
}
