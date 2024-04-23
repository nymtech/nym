import * as React from 'react';
import { Box, Grid, Link, Typography } from '@mui/material';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import { useTheme } from '@mui/material/styles';
import { useNavigate } from 'react-router-dom';
import { WorldMap } from '../../components/WorldMap';
import { useMainContext } from '../../context/main';
import { formatNumber } from '../../utils';
import { BIG_DIPPER } from '../../api/constants';
import { ValidatorsSVG } from '../../icons/ValidatorsSVG';
import { GatewaysSVG } from '../../icons/GatewaysSVG';
import { MixnodesSVG } from '../../icons/MixnodesSVG';
import { Title } from '../../components/Title';
import { ContentCard } from '../../components/ContentCard';
import { StatsCard } from '../../components/StatsCard';
import { Icons } from '../../components/Icons';

export const PageOverview: FCWithChildren = () => {
  const theme = useTheme();
  const navigate = useNavigate();
  const { summaryOverview, gateways, validators, block, countryData, serviceProviders } = useMainContext();
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
                    onClick={() => navigate('/network-components/mixnodes')}
                    title="Mixnodes"
                    icon={<MixnodesSVG />}
                    count={summaryOverview.data?.mixnodes.count || ''}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => navigate('/network-components/mixnodes/active')}
                    title="Active nodes"
                    icon={<Icons.Mixnodes.Status.Active />}
                    color={theme.palette.nym.networkExplorer.mixnodes.status.active}
                    count={summaryOverview.data?.mixnodes.activeset.active}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
                <Grid item xs={12} md={4}>
                  <StatsCard
                    onClick={() => navigate('/network-components/mixnodes/standby')}
                    title="Standby nodes"
                    color={theme.palette.nym.networkExplorer.mixnodes.status.standby}
                    icon={<Icons.Mixnodes.Status.Standby />}
                    count={summaryOverview.data?.mixnodes.activeset.standby}
                    errorMsg={summaryOverview?.error}
                  />
                </Grid>
              </>
            )}
            {gateways && (
              <Grid item xs={12} md={4}>
                <StatsCard
                  onClick={() => navigate('/network-components/gateways')}
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
                <Link
                  href={`${BIG_DIPPER}/blocks`}
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
                  <OpenInNewIcon fontWeight="inherit" fontSize="inherit" sx={{ ml: 0.5 }} />
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
  );
};
