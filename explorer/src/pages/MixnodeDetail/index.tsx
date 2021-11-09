import * as React from 'react';
import {
  Box,
  Grid,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { useMainContext } from 'src/context/main';
import { useParams } from 'react-router-dom';
import { ContentCard } from 'src/components/ContentCard';
import { WorldMap } from 'src/components/WorldMap';
import { BondBreakdownTable } from 'src/components/BondBreakdown';
import { TwoColSmallTable } from 'src/components/TwoColSmallTable';
import { UptimeChart } from 'src/components/UptimeChart';
import { mixnodeToGridRow, scrollToRef } from 'src/utils';
import { ComponentError } from 'src/components/ComponentError';
import { cellStyles } from 'src/components/Universal-DataGrid';
import { MixNodeResponseItem } from 'src/typeDefs/explorer-api';

import { printableCoin } from '@nymproject/nym-validator-client';
import { Title } from 'src/components/Title';
import { UniversalTable } from 'src/components/UniversalTable';

const columns: any = [
  {
    field: 'owner',
    title: 'Owner',
    flex: 1,
    headerAlign: 'left',
    width: 230,
  },
  {
    field: 'identity_key',
    title: 'Identity Key',
    flex: 1,
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

export const PageMixnodeDetail: React.FC = () => {
  const ref = React.useRef();
  const [row, setRow] = React.useState<MixNodeResponseItem[]>([]);
  const {
    fetchMixnodeById,
    mixnodeDetailInfo,
    fetchStatsById,
    fetchDelegationsById,
    fetchUptimeStoryById,
    fetchStatusById,
    stats,
    status,
    uptimeStory,
  } = useMainContext();
  const { id }: any = useParams();

  React.useEffect(() => {
    const hasNoDetail = id && !mixnodeDetailInfo;
    const hasIncorrectDetail =
      id &&
      mixnodeDetailInfo?.data &&
      mixnodeDetailInfo?.data[0].mix_node.identity_key !== id;
    if (hasNoDetail || hasIncorrectDetail) {
      fetchMixnodeById(id);
      fetchDelegationsById(id);
      fetchStatsById(id);
      fetchStatusById(id);
      fetchUptimeStoryById(id);
    } else if (mixnodeDetailInfo?.data !== undefined) {
      setRow(mixnodeDetailInfo?.data);
    }
  }, [id, mixnodeDetailInfo]);

  React.useEffect(() => {
    scrollToRef(ref);
  }, [ref]);

  return (
    <>
      <Box component="main" ref={ref}>
        <Grid container spacing={2}>
          <Grid item xs={12}>
            <Title text="Mixnode Detail" />
          </Grid>
        </Grid>

        <Grid container>
          <Grid item xs={12}>
            <UniversalTable
              columnsData={columns}
              tableName="Mixnode detail table"
              rows={mixnodeToGridRow(row)}
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
                      stats?.data?.packets_explicitly_dropped_since_startup ||
                        0,
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
                      stats?.data
                        ?.packets_explicitly_dropped_since_last_update || 0,
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
                    (status?.data?.ports &&
                      Object.values(status.data.ports)) || [false, false, false]
                  }
                />
              </ContentCard>
            )}
          </Grid>
          <Grid item xs={12} md={8}>
            {mixnodeDetailInfo && (
              <ContentCard title="Location">
                {mixnodeDetailInfo?.error && (
                  <ComponentError text="There was a problem retrieving this mixnode location" />
                )}
                {mixnodeDetailInfo.data &&
                  mixnodeDetailInfo?.data[0]?.location && (
                    <WorldMap
                      loading={mixnodeDetailInfo.isLoading}
                      userLocation={[
                        mixnodeDetailInfo?.data[0]?.location?.lng,
                        mixnodeDetailInfo?.data[0]?.location?.lat,
                      ]}
                    />
                  )}
              </ContentCard>
            )}
          </Grid>
        </Grid>
      </Box>
    </>
  );
};
