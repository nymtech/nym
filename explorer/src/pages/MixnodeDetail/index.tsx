import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
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
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { MixNodeResponseItem } from 'src/typeDefs/explorer-api';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { printableCoin } from '@nymproject/nym-validator-client';
import { Title } from 'src/components/Title';

const columns: GridColDef[] = [
  {
    field: 'owner',
    renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
    width: 200,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    renderCell: (params: GridRenderCellParams) => (
      <div>
        <Typography sx={cellStyles}>{params.value}</Typography>
      </div>
    ),
  },
  {
    field: 'identity_key',
    renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
    width: 200,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    renderCell: (params: GridRenderCellParams) => (
      <div>
        <Typography sx={cellStyles}>{params.value}</Typography>
      </div>
    ),
  },
  {
    field: 'bond',
    headerName: 'Bond',
    type: 'number',
    renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
    flex: 1,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    renderCell: (params: GridRenderCellParams) => {
      const bondAsPunk = printableCoin({
        amount: params.value as string,
        denom: 'upunk',
      });
      return <Typography sx={cellStyles}>{bondAsPunk}</Typography>;
    },
  },
  {
    field: 'self_percentage',
    headerName: 'Self %',
    headerAlign: 'left',
    type: 'number',
    width: 99,
    headerClassName: 'MuiDataGrid-header-override',
    renderHeader: () => <CustomColumnHeading headingTitle="Self %" />,
    renderCell: (params: GridRenderCellParams) => (
      <div>
        <Typography sx={cellStyles}>{params.value}%</Typography>
      </div>
    ),
  },
  {
    field: 'host',
    renderHeader: () => <CustomColumnHeading headingTitle="IP:Port" />,
    flex: 1,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    renderCell: (params: GridRenderCellParams) => (
      <div>
        <Typography sx={cellStyles}>{params.value}</Typography>
      </div>
    ),
  },
  {
    field: 'location',
    renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
    flex: 1,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    renderCell: (params: GridRenderCellParams) => (
      <div>
        <Typography sx={cellStyles} data-testid="location-value">
          {params.value}
        </Typography>
      </div>
    ),
  },
  {
    field: 'layer',
    renderHeader: () => <CustomColumnHeading headingTitle="Layer" />,
    flex: 1,
    headerAlign: 'left',
    headerClassName: 'MuiDataGrid-header-override',
    type: 'number',
    renderCell: (params: GridRenderCellParams) => (
      <Typography sx={cellStyles} data-testid="node-layer">
        {params.value}
      </Typography>
    ),
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
            {mixnodeDetailInfo && (
              <ContentCard>
                <UniversalDataGrid
                  columnsData={columns}
                  rows={mixnodeToGridRow(row)}
                  loading={mixnodeDetailInfo.isLoading}
                  pageSize="1"
                  pagination={false}
                  hideFooter
                />
              </ContentCard>
            )}
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
