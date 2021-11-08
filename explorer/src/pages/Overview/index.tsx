import * as React from 'react';
import { Box, Grid, Link } from '@mui/material';
import { WorldMap } from 'src/components/WorldMap';
import { useHistory } from 'react-router-dom';
import { useMainContext } from 'src/context/main';
import { formatNumber } from 'src/utils';
import { BIG_DIPPER } from 'src/api/constants';
import { ValidatorsSVG } from 'src/icons/ValidatorsSVG';
import { GatewaysSVG } from 'src/icons/GatewaysSVG';
import { MixnodesSVG } from 'src/icons/MixnodesSVG';
import { Title } from 'src/components/Title';
import { ContentCard } from '../../components/ContentCard';
import { StatsCard } from '../../components/StatsCard';

export const PageOverview: React.FC = () => {
  const history = useHistory();
  const { mixnodes, gateways, validators, block, countryData } =
    useMainContext();
  return (
    <>
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid>
          <Grid item>
            <Title text="Overview" />
          </Grid>
          <Grid item>
            <Grid container spacing={2}>
              {mixnodes && (
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => history.push('/network-components/mixnodes')}
                    title="Mixnodes"
                    icon={<MixnodesSVG />}
                    count={mixnodes?.data?.length || ''}
                    errorMsg={mixnodes?.error}
                  />
                </Grid>
              )}
              {gateways && (
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => history.push('/network-components/gateways')}
                    title="Gateways"
                    count={gateways?.data?.length || ''}
                    errorMsg={gateways?.error}
                    icon={<GatewaysSVG />}
                  />
                </Grid>
              )}

              {validators && (
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => window.open(`${BIG_DIPPER}/validators`)}
                    title="Validators"
                    count={validators?.data?.count || ''}
                    errorMsg={validators?.error}
                    icon={<ValidatorsSVG />}
                  />
                </Grid>
              )}
              {block?.data && (
                <Grid item xs={12}>
                  <ContentCard
                    title={
                      <Link
                        href={`${BIG_DIPPER}/blocks`}
                        target="_blank"
                        rel="noreferrer"
                        underline="none"
                        color="inherit"
                      >
                        Current block height is {formatNumber(block.data)}
                      </Link>
                    }
                  />
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
    </>
  );
};
