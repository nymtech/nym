import * as React from 'react';
import { Alert, AlertTitle, Box, CircularProgress, Grid, Typography } from '@mui/material';
import { useParams } from 'react-router-dom';
import { GatewayBond } from '../../typeDefs/explorer-api';
import { ColumnsType, DetailTable } from '../../components/DetailTable';
import { gatewayEnrichedToGridRow, GatewayEnrichedRowType } from '../../components/Gateways';
import { ComponentError } from '../../components/ComponentError';
import { ContentCard } from '../../components/ContentCard';
import { TwoColSmallTable } from '../../components/TwoColSmallTable';
import { UptimeChart } from '../../components/UptimeChart';
import { GatewayContextProvider, useGatewayContext } from '../../context/gateway';
import { useMainContext } from '../../context/main';
import { Title } from '../../components/Title';
import { ReleaseAlert } from '@src/components/ReleaseAlert';

const columns: ColumnsType[] = [
  {
    field: 'identity_key',
    title: 'Identity Key',
    headerAlign: 'left',
    width: 230,
  },
  {
    field: 'bond',
    title: 'Bond',
    headerAlign: 'left',
  },
  {
    field: 'node_performance',
    title: 'Routing Score',
    headerAlign: 'left',
    tooltipInfo:
      "Gateway's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test",
  },
  {
    field: 'avgUptime',
    title: 'Avg. Score',
    headerAlign: 'left',
    tooltipInfo: "Gateway's average routing score in the last 24 hours",
  },
  {
    field: 'host',
    title: 'IP',
    headerAlign: 'left',
    width: 99,
  },
  {
    field: 'location',
    title: 'Location',
    headerAlign: 'left',
  },
  {
    field: 'owner',
    title: 'Owner',
    headerAlign: 'left',
  },
  {
    field: 'version',
    title: 'Version',
    headerAlign: 'left',
  },
];

/**
 * Shows gateway details
 */
const PageGatewayDetailsWithState = ({ selectedGateway }: { selectedGateway: GatewayBond | undefined }) => {
  const [enrichGateway, setEnrichGateway] = React.useState<GatewayEnrichedRowType>();
  const [status, setStatus] = React.useState<number[] | undefined>();
  const { uptimeReport, uptimeStory } = useGatewayContext();

  React.useEffect(() => {
    if (uptimeReport?.data && selectedGateway) {
      setEnrichGateway(gatewayEnrichedToGridRow(selectedGateway, uptimeReport.data));
    }
  }, [uptimeReport, selectedGateway]);

  React.useEffect(() => {
    if (enrichGateway) {
      setStatus([enrichGateway.mixPort, enrichGateway.clientsPort]);
    }
  }, [enrichGateway]);

  return (
    <Box component="main">
      <ReleaseAlert />
      <Title text="Gateway Detail" />
      <Grid container>
        <Grid item xs={12}>
          <DetailTable
            columnsData={columns}
            tableName="Gateway detail table"
            rows={enrichGateway ? [enrichGateway] : []}
          />
        </Grid>
      </Grid>

      <Grid container spacing={2} mt={0}>
        <Grid item xs={12} md={4}>
          {status && (
            <ContentCard title="Gateway Status">
              <TwoColSmallTable
                loading={false}
                keys={['Mix port', 'Client WS API Port']}
                values={status.map((each) => each)}
                icons={status.map((elem) => !!elem)}
              />
            </ContentCard>
          )}
        </Grid>
        <Grid item xs={12} md={8}>
          {uptimeStory && (
            <ContentCard title="Routing Score">
              {uptimeStory.error && <ComponentError text="There was a problem retrieving routing score." />}
              <UptimeChart
                loading={uptimeStory.isLoading}
                xLabel="Date"
                yLabel="Daily average"
                uptimeStory={uptimeStory}
              />
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
const PageGatewayDetailGuard: FCWithChildren = () => {
  const [selectedGateway, setSelectedGateway] = React.useState<GatewayBond>();
  const { gateways } = useMainContext();
  const { id } = useParams<{ id: string | undefined }>();

  React.useEffect(() => {
    if (gateways?.data) {
      setSelectedGateway(gateways.data.find((g) => g.gateway.identity_key === id));
    }
  }, [gateways, id]);

  if (gateways?.isLoading) {
    return <CircularProgress />;
  }

  if (gateways?.error) {
    // eslint-disable-next-line no-console
    console.error(gateways?.error);
    return (
      <Alert severity="error">
        Oh no! Could not load mixnode <code>{id || ''}</code>
      </Alert>
    );
  }

  // loaded, but not found
  if (gateways && !gateways.isLoading && !gateways.data) {
    return (
      <Alert severity="warning">
        <AlertTitle>Gateway not found</AlertTitle>
        Sorry, we could not find a mixnode with id <code>{id || ''}</code>
      </Alert>
    );
  }

  return <PageGatewayDetailsWithState selectedGateway={selectedGateway} />;
};

/**
 * Wrapper component that adds the mixnode content based on the `id` in the address URL
 */
export const PageGatewayDetail: FCWithChildren = () => {
  const { id } = useParams<{ id: string | undefined }>();

  if (!id) {
    return <Alert severity="error">Oh no! No mixnode identity key specified</Alert>;
  }

  return (
    <GatewayContextProvider gatewayIdentityKey={id}>
      <PageGatewayDetailGuard />
    </GatewayContextProvider>
  );
};
