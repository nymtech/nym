import * as React from 'react';
import { Alert, AlertTitle, Box, CircularProgress, Grid, Typography } from '@mui/material';
import { useParams } from 'react-router-dom';
import { GatewayResponseItem } from '../../typeDefs/explorer-api';
import { ColumnsType, DetailTable } from '../../components/DetailTable';
import { gatewayEnrichedToGridRow, GatewayEnridedRowType } from '../../components/Gateways';
import { ComponentError } from '../../components/ComponentError';
import { ContentCard } from '../../components/ContentCard';
import { TwoColSmallTable } from '../../components/TwoColSmallTable';
import { UptimeChart } from '../../components/UptimeChart';
// import { GatewayDetailSection } from '../../components/Gateways/DetailSection';
import { GatewayContextProvider, useGatewayContext } from '../../context/gateway';
import { useMainContext } from '../../context/main';
import { Title } from '../../components/Title';

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
    flex: 1,
    headerAlign: 'left',
  },
  {
    field: 'routing_score',
    title: 'Routing Score',
    flex: 1,
    headerAlign: 'left',
    tooltipInfo: 'Estimated reward per epoch for this profit margin if your node is selected in the active set.',
  },
  {
    field: 'avg_uptime',
    title: 'Avg. Score',
    flex: 1,
    headerAlign: 'left',
    tooltipInfo: 'Estimated reward per epoch for this profit margin if your node is selected in the active set.',
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
    flex: 1,
  },
  {
    field: 'owner',
    title: 'Owner',
    headerAlign: 'left',
    flex: 1,
  },
];

/**
 * Shows gateway details
 */
const PageGatewayDetailsWithState: React.FC<{ selectedGateway: GatewayResponseItem | undefined }> = ({
  selectedGateway,
}) => {
  const [enrichGateway, setEnrichGateway] = React.useState<GatewayEnridedRowType>();
  const [status, setStatus] = React.useState<number[] | undefined>();
  const { uptimeReport } = useGatewayContext();

  React.useEffect(() => {
    if (uptimeReport?.data && selectedGateway) {
      setEnrichGateway(gatewayEnrichedToGridRow(selectedGateway, uptimeReport.data));
    }
  }, [uptimeReport, selectedGateway]);

  React.useEffect(() => {
    if (enrichGateway) {
      setStatus([enrichGateway?.mix_port, enrichGateway?.clients_port]);
    }
  }, [enrichGateway]);

  return (
    <Box component="main">
      <Title text="Gateway Detail" />

      <Grid container spacing={2} mt={1} mb={6}>
        <Grid item xs={12}>
          Gateway name & description
          {/* {gatewayRow && description?.data && (
            <GatewayDetailSection gatewayRow={gatewayRow} mixnodeDescription={description.data} />
          )} */}
        </Grid>
      </Grid>

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
                icons={status.map((elem) => !!elem) || [false, false]}
              />
            </ContentCard>
          )}
        </Grid>
        <Grid item xs={12} md={8}>
          {/* {uptimeStory && (
            <ContentCard title="Routing Score">
              {uptimeStory.error && <ComponentError text="There was a problem retrieving routing score." />}
              <UptimeChart loading={uptimeStory.isLoading} xLabel="date" uptimeStory={uptimeStory} />
            </ContentCard>
          )} */}
        </Grid>
      </Grid>
    </Box>
  );
};

/**
 * Guard component to handle loading and not found states
 */
const PageGatewayDetailGuard: React.FC = () => {
  const [selectedGateway, setSelectedGateway] = React.useState<GatewayResponseItem | undefined>();
  const { gateways } = useMainContext();
  const { id } = useParams<{ id: string | undefined }>();

  React.useEffect(() => {
    if (gateways?.data) {
      setSelectedGateway(gateways.data.find((gateway) => gateway.gateway.identity_key === id));
    }
  }, [gateways]);

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
export const PageGatewayDetail: React.FC = () => {
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
