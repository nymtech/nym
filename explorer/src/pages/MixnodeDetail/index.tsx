import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { Box, Grid, Typography } from '@mui/material';
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
            <TableContainer component={Paper}>
              <Table sx={{ minWidth: 650 }} aria-label="mixnode detail table">
                <TableHead>
                  <TableRow>
                    <TableCell sx={{ fontWeight: 'bold' }}>Owner</TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      Identity Key
                    </TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      Bond&nbsp;
                    </TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      Self %&nbsp;
                    </TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      IP:Port&nbsp;
                    </TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      Location&nbsp;
                    </TableCell>
                    <TableCell sx={{ fontWeight: 'bold' }}>
                      Layer&nbsp;
                    </TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {mixnodeToGridRow(row).map((eachRow) => (
                    <TableRow
                      key={eachRow.owner}
                      sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                    >
                      <TableCell
                        component="th"
                        scope="row"
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 230,
                          maxWidth: 230,
                          width: 230,
                        }}
                      >
                        {eachRow.owner}
                      </TableCell>
                      <TableCell
                        component="th"
                        scope="row"
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 220,
                          maxWidth: 220,
                          width: 220,
                        }}
                      >
                        {eachRow.identity_key}
                      </TableCell>
                      <TableCell
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 140,
                          maxWidth: 140,
                          width: 140,
                        }}
                      >
                        {printableCoin({
                          amount: eachRow.bond.toString(),
                          denom: 'upunk',
                        })}
                      </TableCell>
                      <TableCell
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 90,
                          maxWidth: 90,
                          width: 90,
                        }}
                      >
                        {eachRow.self_percentage}
                      </TableCell>
                      <TableCell
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 120,
                          maxWidth: 120,
                          width: 120,
                        }}
                      >
                        {eachRow.host}
                      </TableCell>
                      <TableCell
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 150,
                          maxWidth: 150,
                          width: 150,
                        }}
                      >
                        {eachRow.location}
                      </TableCell>
                      <TableCell
                        sx={{
                          ...cellStyles,
                          padding: 2,
                          minWidth: 200,
                          maxWidth: 200,
                          width: 200,
                        }}
                      >
                        {eachRow.layer}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
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
