import * as React from 'react';
import { Alert, AlertTitle, Box, CircularProgress, Grid, Typography } from '@mui/material';
import { useParams } from 'react-router-dom';
import { ColumnsType, DetailTable } from '../../components/DetailTable';
import { BondBreakdownTable } from '../../components/MixNodes/BondBreakdown';
import { DelegatorsInfoTable, EconomicsInfoColumns, EconomicsInfoRows } from '../../components/MixNodes/Economics';
import { ComponentError } from '../../components/ComponentError';
import { ContentCard } from '../../components/ContentCard';
import { TwoColSmallTable } from '../../components/TwoColSmallTable';
import { UptimeChart } from '../../components/UptimeChart';
import { WorldMap } from '../../components/WorldMap';
import { MixNodeDetailSection } from '../../components/MixNodes/DetailSection';
import { MixnodeContextProvider, useMixnodeContext } from '../../context/mixnode';
import { Title } from '../../components/Title';

const columns: ColumnsType[] = [
  {
    field: 'owner',
    title: 'Owner',
    width: '15%',
  },
  {
    field: 'identity_key',
    title: 'Identity Key',
    width: '15%',
  },

  {
    field: 'bond',
    title: 'Stake',
    width: '12.5%',
  },
  {
    field: 'stake_saturation',
    title: 'Stake Saturation',
    width: '12.5%',
    tooltipInfo:
      'Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is 730k NYM, computed as S/K where S is target amount of tokens staked in the network and K is the number of nodes in the reward set.',
  },
  {
    field: 'self_percentage',
    width: '10%',
    title: 'Bond %',
  },

  {
    field: 'host',
    width: '10%',
    title: 'Host',
  },
  {
    field: 'location',
    title: 'Location',
  },

  {
    field: 'layer',
    title: 'Layer',
  },
];

/**
 * Shows mix node details
 */
const PageMixnodeDetailWithState: FCWithChildren = () => {
  const { mixNode, mixNodeRow, description, stats, status, uptimeStory, uniqDelegations } = useMixnodeContext();
  console.log(mixNodeRow);
  return (
    <Box component="main">
      <Title text="Mixnode Detail" />
      <Grid container spacing={2} mt={1} mb={6}>
        <Grid item xs={12}>
          {mixNodeRow && description?.data && (
            <MixNodeDetailSection mixNodeRow={mixNodeRow} mixnodeDescription={description.data} />
          )}
        </Grid>
      </Grid>
      <Grid container>
        <Grid item xs={12}>
          <DetailTable columnsData={columns} tableName="Mixnode detail table" rows={mixNodeRow ? [mixNodeRow] : []} />
        </Grid>
      </Grid>
      <Grid container spacing={2} mt={0}>
        <Grid item xs={12}>
          <DelegatorsInfoTable
            columnsData={EconomicsInfoColumns}
            tableName="Delegators info table"
            rows={[EconomicsInfoRows()]}
          />
        </Grid>
      </Grid>
      <Grid container spacing={2} mt={0}>
        <Grid item xs={12}>
          <ContentCard title={`Stake Breakdown (${uniqDelegations?.data?.length} delegators)`}>
            <BondBreakdownTable />
          </ContentCard>
        </Grid>
      </Grid>
      <Grid container spacing={2} mt={0}>
        <Grid item xs={12} md={4}>
          <ContentCard title="Mixnode Stats">
            {stats && (
              <>
                {stats.error && <ComponentError text="There was a problem retrieving this nodes stats." />}
                <TwoColSmallTable
                  loading={stats.isLoading}
                  error={stats?.error?.message}
                  title="Since startup"
                  keys={['Received', 'Sent', 'Explicitly dropped']}
                  values={[
                    stats?.data?.packets_received_since_startup || 0,
                    stats?.data?.packets_sent_since_startup || 0,
                    stats?.data?.packets_explicitly_dropped_since_startup || 0,
                  ]}
                />
                <TwoColSmallTable
                  loading={stats.isLoading}
                  error={stats?.error?.message}
                  title="Since last update"
                  keys={['Received', 'Sent', 'Explicitly dropped']}
                  values={[
                    stats?.data?.packets_received_since_last_update || 0,
                    stats?.data?.packets_sent_since_last_update || 0,
                    stats?.data?.packets_explicitly_dropped_since_last_update || 0,
                  ]}
                  marginBottom
                />
              </>
            )}
            {!stats && <Typography>No stats information</Typography>}
          </ContentCard>
        </Grid>
        <Grid item xs={12} md={8}>
          {uptimeStory && (
            <ContentCard title="Routing Score">
              {uptimeStory.error && <ComponentError text="There was a problem retrieving routing score." />}
              <UptimeChart loading={uptimeStory.isLoading} xLabel="date" uptimeStory={uptimeStory} />
            </ContentCard>
          )}
        </Grid>
      </Grid>
      <Grid container spacing={2} mt={0}>
        <Grid item xs={12} md={4}>
          {status && (
            <ContentCard title="Mixnode Status">
              {status.error && <ComponentError text="There was a problem retrieving port information" />}
              <TwoColSmallTable
                loading={status.isLoading}
                error={status?.error?.message}
                keys={['Mix port', 'Verloc port', 'HTTP port']}
                values={[1789, 1790, 8000].map((each) => each)}
                icons={(status?.data?.ports && Object.values(status.data.ports)) || [false, false, false]}
              />
            </ContentCard>
          )}
        </Grid>
        <Grid item xs={12} md={8}>
          {mixNode && (
            <ContentCard title="Location">
              {mixNode?.error && <ComponentError text="There was a problem retrieving this mixnode location" />}
              {mixNode?.data?.location?.latitude && mixNode?.data?.location?.longitude && (
                <WorldMap
                  loading={mixNode.isLoading}
                  userLocation={[mixNode.data.location.longitude, mixNode.data.location.latitude]}
                />
              )}
            </ContentCard>
          )}
        </Grid>
      </Grid>
    </Box>
  );
};

/**
 * Guard component to handle loading and not found states
 */
const PageMixnodeDetailGuard: FCWithChildren = () => {
  const { mixNode } = useMixnodeContext();
  const { id } = useParams<{ id: string | undefined }>();

  if (mixNode?.isLoading) {
    return <CircularProgress />;
  }

  if (mixNode?.error) {
    // eslint-disable-next-line no-console
    console.error(mixNode?.error);
    return (
      <Alert severity="error">
        Oh no! Could not load mixnode <code>{id || ''}</code>
      </Alert>
    );
  }

  // loaded, but not found
  if (mixNode && !mixNode.isLoading && !mixNode.data) {
    return (
      <Alert severity="warning">
        <AlertTitle>Mixnode not found</AlertTitle>
        Sorry, we could not find a mixnode with id <code>{id || ''}</code>
      </Alert>
    );
  }

  return <PageMixnodeDetailWithState />;
};

/**
 * Wrapper component that adds the mixnode content based on the `id` in the address URL
 */
export const PageMixnodeDetail: FCWithChildren = () => {
  const { id } = useParams<{ id: string | undefined }>();

  if (!id) {
    return <Alert severity="error">Oh no! No mixnode identity key specified</Alert>;
  }

  return (
    <MixnodeContextProvider mixId={id}>
      <PageMixnodeDetailGuard />
    </MixnodeContextProvider>
  );
};
