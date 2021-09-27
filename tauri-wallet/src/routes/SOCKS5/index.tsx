import { Grid } from '@material-ui/core'
import React, { useContext } from 'react'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { Layout } from '../../layouts'
import { MainCard, TopCard } from './Cards'
import { DownloadCard, LimitCard, UploadCard } from './DataCards'

export const Socks5 = () => {
  const { ss5IsActive, toggleSs5 } = useContext(ClientContext)

  return (
    <Layout>
      <NymCard title="SOCKS5" style={{ width: '100%' }}>
        <Grid container spacing={6}>
          <Grid item xs={12}>
            <TopCard isActive={ss5IsActive} toggleIsActive={toggleSs5} />
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
      </NymCard>
    </Layout>
  )
}
