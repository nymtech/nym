import React from 'react';
import { Box, Grid, IconButton, Typography } from '@mui/material';
import {
  SettingsAccessibility as ConnectIcon,
  ArrowForwardSharp,
} from '@mui/icons-material';
import { WorldMap } from 'src/components/WorldMap';
import { useHistory } from 'react-router-dom';
import { ApiDataContext } from 'src/context/api';
import { ContentCard } from '../../components/ContentCard';

export const PageOverview: React.FC = () => {
  const history = useHistory();
  const stats: any = React.useContext(ApiDataContext);
  return (
    <>
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid container spacing={0}>
          <Grid item xs={12}>
            <Typography sx={{ marginLeft: (theme) => theme.spacing(3) }}>
              Overview
            </Typography>
          </Grid>

          <Grid item xs={12} md={4} lg={4}>
            <ContentCard
              title="Mixnodes"
              subtitle={JSON.stringify(stats?.mixnodes?.length) || '0'}
              Icon={<ConnectIcon />}
              Action={
                <IconButton>
                  <ArrowForwardSharp
                    onClick={() => history.push('/network-components/mixnodes')}
                  />
                </IconButton>
              }
            />
          </Grid>
          <Grid item xs={12} md={4} lg={4}>
            <ContentCard
              title="Gateways"
              subtitle={JSON.stringify(stats?.gateways?.length) || '0'}
              Icon={<ConnectIcon />}
              Action={
                <IconButton>
                  <ArrowForwardSharp
                    onClick={() => history.push('/network-components/gateways')}
                  />
                </IconButton>
              }
            />
          </Grid>
          <Grid item xs={12} md={4} lg={4}>
            <ContentCard
              title="Validators"
              subtitle={stats?.validators || '0'}
              Icon={<ConnectIcon />}
              Action={
                <IconButton>
                  <ArrowForwardSharp
                    onClick={() => history.push('/network-components/mixnodes')}
                  />
                </IconButton>
              }
            />
          </Grid>

          <Grid item xs={12}>
            <ContentCard title="Current block height is 647,059" />
          </Grid>

          <Grid item xs={12}>
            <WorldMap />
          </Grid>
        </Grid>
      </Box>
    </>
  );
};
