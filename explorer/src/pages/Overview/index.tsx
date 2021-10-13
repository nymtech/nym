import React from 'react';
import { Box, Grid, IconButton, Typography } from '@mui/material';
import {
  SettingsAccessibility as ConnectIcon,
  ArrowForwardSharp,
} from '@mui/icons-material';
import { WorldMap } from 'src/components/WorldMap';
import { useHistory } from 'react-router-dom';
import { MainContext } from 'src/context/main';
import { formatNumber } from 'src/utils';
import { ContentCard } from '../../components/ContentCard';
import { BIG_DIPPER } from 'src/api/constants';

export const PageOverview: React.FC = () => {
  const history = useHistory();
  const { mixnodes, gateways, validators, block, countryData }: any =
    React.useContext(MainContext);
  return (
    <>
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid container spacing={2}>
          <Grid item xs={12}>
            <Typography sx={{ marginLeft: 3 }}>
              Overview
            </Typography>
          </Grid>

          {mixnodes && (
            <Grid item xs={12} md={4} lg={4}>
              <ContentCard
                onClick={() =>
                  history.push('/network-components/mixnodes')
                }
                title="Mixnodes"
                subtitle={mixnodes?.data?.length || ''}
                errorMsg={mixnodes?.error}
                Icon={<ConnectIcon />}
                Action={
                  <IconButton>
                    <ArrowForwardSharp />
                  </IconButton>
                }
              />
            </Grid>
          )}
          {gateways && (
            <Grid item xs={12} md={4} lg={4}>
              <ContentCard
                onClick={() =>
                  history.push('/network-components/gateways')
                }
                title="Gateways"
                subtitle={gateways?.data?.length || ''}
                errorMsg={gateways?.error}
                Icon={<ConnectIcon />}
                Action={
                  <IconButton>
                    <ArrowForwardSharp />
                  </IconButton>
                }
              />
            </Grid>
          )}
          {validators && (
            <Grid item xs={12} md={4} lg={4}>
              <ContentCard
                onClick={() => window.open(`${BIG_DIPPER}/validators`)}
                title='Validators'
                subtitle={validators?.data?.count || ''}
                errorMsg={validators?.error}
                Icon={<ConnectIcon />}
                Action={
                  <IconButton>
                    <ArrowForwardSharp />
                  </IconButton>
                }
              />
            </Grid>
          )}

          <Grid item xs={12}>
            <ContentCard
              title={
                <a
                  href={`${BIG_DIPPER}/blocks`}
                  target="_blank" style={{
                    textDecoration: 'none',
                    color: 'white',
                  }}
                >
                  Current block height is {formatNumber(block?.data)}
                </a>
              }
            />
          </Grid>

          <Grid item xs={12}>
            <ContentCard title="Distribution of nodes around the world">
              <WorldMap
                loading={false}
                countryData={countryData}
              />
            </ContentCard>
          </Grid>
        </Grid>
      </Box>
    </>
  );
};
