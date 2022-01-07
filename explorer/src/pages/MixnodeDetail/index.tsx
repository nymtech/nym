import * as React from 'react';
import {
  Alert,
  AlertTitle,
  Box,
  CircularProgress,
  Grid,
  Typography,
} from '@mui/material';
import { ColumnsType, DetailTable } from 'src/components/DetailTable';
import { useParams } from 'react-router-dom';
import { BondBreakdownTable } from 'src/components/MixNodes/BondBreakdown';
import { ComponentError } from 'src/components/ComponentError';
import { ContentCard } from 'src/components/ContentCard';
import { TwoColSmallTable } from 'src/components/TwoColSmallTable';
import { UptimeChart } from 'src/components/UptimeChart';
import { WorldMap } from 'src/components/WorldMap';
import { MixNodeDetailSection } from '../../components/MixNodes/DetailSection';
import {
  MixnodeContextProvider,
  useMixnodeContext,
} from '../../context/mixnode';
import { Title } from '../../components/Title';

const columns: ColumnsType[] = [
  {
    field: 'owner',
    title: 'Owner',
    headerAlign: 'left',
    width: 230,
  },
  {
    field: 'identity_key',
    title: 'Identity Key',
    headerAlign: 'left',
    width: 230,
  },

  {
    field: 'bond',
    title: 'Bond',
    flex: 1,
    headerAlign: 'left',
  },
  {
    field: 'self_percentage',
    title: 'Self %',
    headerAlign: 'left',
    width: 99,
  },
  {
    field: 'host',
    title: 'Host',
    headerAlign: 'left',
    flex: 1,
  },
  {
    field: 'location',
    title: 'Location',
    headerAlign: 'left',
    flex: 1,
  },
  {
    field: 'layer',
    title: 'Layer',
    headerAlign: 'left',
    flex: 1,
  },
];

/**
 * Wrapper component that adds the mixnode content based on the `id` in the address URL
 */
export const PageMixnodeDetail: React.FC = () => {
  const { id } = useParams<{ id: string | undefined }>();

  if (!id) {
    return (
      <Alert severity="error">Oh no! No mixnode identity key specified</Alert>
    );
  }

  return (
    <MixnodeContextProvider mixNodeIdentityKey={id}>
      <PageMixnodeDetailGuard />
    </MixnodeContextProvider>
  );
};

/**
 * Guard component to handle loading and not found states
 */
const PageMixnodeDetailGuard: React.FC = () => {
  const { mixNode } = useMixnodeContext();
  const { id } = useParams<{ id: string | undefined }>();

  if (mixNode?.isLoading) {
    return <CircularProgress />;
  }

  if (mixNode?.error) {
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
 * Shows mix node details
 */
const PageMixnodeDetailWithState: React.FC = () => {
  const { mixNode, mixNodeRow, description, stats, status, uptimeStory } =
    useMixnodeContext();

  return (
    <Box component="main">
      <Title text="Mixnode Detail" />

      <Grid container spacing={2} mt={1} mb={6}>
        <Grid item xs={12}>
          {mixNodeRow && description?.data && (
            <MixNodeDetailSection
              mixNodeRow={mixNodeRow}
              mixnodeDescription={description.data}
            />
          )}
        </Grid>
      </Grid>

      <Grid container>
        <Grid item xs={12}>
          <DetailTable
            columnsData={columns}
            tableName="Mixnode detail table"
            rows={mixNodeRow ? [mixNodeRow] : []}
          />
        </Grid>
      </Grid>

      <Grid container spacing={2} mt={0}>
        <Grid item xs={12}>
          <ContentCard title="Bond Breakdown">
            <BondBreakdownTable />
          </ContentCard>
        </Grid>
      </Grid>

      <Grid container spacing={2} mt={0}>
        <Grid item xs={12} md={4}>
          <ContentCard title="Mixnode Stats">
            {stats && (
              <>
                {stats.error && (
                  <ComponentError text="There was a problem retrieving this nodes stats." />
                )}
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
                    stats?.data?.packets_explicitly_dropped_since_last_update ||
                      0,
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
            <ContentCard title="Uptime story">
              {uptimeStory.error && (
                <ComponentError text="There was a problem retrieving uptime history." />
              )}
              <UptimeChart
                loading={uptimeStory.isLoading}
                xLabel="date"
                yLabel="uptime"
                uptimeStory={uptimeStory}
              />
            </ContentCard>
          )}
        </Grid>
      </Grid>

      <Grid container spacing={2} mt={0}>
        <Grid item xs={12} md={4}>
          {status && (
            <ContentCard title="Mixnode Status">
              {status.error && (
                <ComponentError text="There was a problem retrieving port information" />
              )}
              <TwoColSmallTable
                loading={status.isLoading}
                error={status?.error?.message}
                keys={['Mix port', 'Verloc port', 'HTTP port']}
                values={[1789, 1790, 8000].map((each) => each)}
                icons={
                  (status?.data?.ports && Object.values(status.data.ports)) || [
                    false,
                    false,
                    false,
                  ]
                }
              />
            </ContentCard>
          )}
        </Grid>
        <Grid item xs={12} md={8}>
          {mixNode && (
            <ContentCard title="Location">
              {mixNode?.error && (
                <ComponentError text="There was a problem retrieving this mixnode location" />
              )}
              {mixNode.data && mixNode?.data?.location && (
                <WorldMap
                  loading={mixNode.isLoading}
                  userLocation={[
                    mixNode?.data?.location?.lng,
                    mixNode?.data?.location?.lat,
                  ]}
                />
              )}
            </ContentCard>
          )}
        </Grid>
      </Grid>
    </Box>
  );
};
