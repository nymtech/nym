import React, { useRef } from 'react';
import { ArrowForward, CheckCircleOutline, Description, Download } from '@mui/icons-material';
import { Box, Button, Card, Chip, CircularProgress, Divider, Grid, Stack, Typography } from '@mui/material';
import format from 'date-fns/format';
import { NodePath } from 'src/svg-icons/node-path';
import { useReactToPrint } from 'react-to-print';
import { ResultsCard } from '../components/ResultsCard';
import { ResultsCardDetail } from '../components/ResultsCardDetail';

export type Layer = '1' | '2' | '3' | 'gateway';

const getLayerDescription = (layer: Layer) => {
  if (layer === 'gateway') return 'Your node was in the Gateway layer';
  return `Your node was in layer ${layer}`;
};

export const NodeSpeed = ({ Mbps, performance }: { Mbps: number; performance: 'poor' | 'fair' | 'good' }) => (
  <ResultsCard
    label="Node speed"
    detail={`${performance === 'good' ? 'Fast' : performance === 'poor' ? 'Slow' : 'Fair'} node`}
    isOk={performance === 'good'}
  >
    <Box
      sx={{
        display: 'flex',
        position: 'relative',
        width: 250,
        height: 250,
        justifyContent: 'center',
        alignItems: 'center',
        mx: 'auto',
        mt: 4,
      }}
    >
      <CircularProgress
        variant="determinate"
        value={performance === 'poor' ? 12.5 : performance === 'good' ? 85 : 65}
        size={250}
        sx={{ position: 'absolute', top: 0, left: 0 }}
        color={performance === 'poor' ? 'error' : performance === 'good' ? 'success' : 'warning'}
      />
      <Stack alignItems="center" gap={1}>
        <Typography fontWeight="bold" variant="h4">
          {Mbps}
        </Typography>
        <Typography>Mbps</Typography>
      </Stack>
    </Box>
  </ResultsCard>
);

export const Packets = ({ sent, received }: { sent: string; received: string }) => {
  const percentage = Math.round((+received / +sent) * 100);
  return (
    <ResultsCard label="Packets" detail={`${percentage}% packets`} isOk={percentage > 75}>
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets sent" detail={sent} />
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets received" detail={received} />
    </ResultsCard>
  );
};

export const Path = ({ layer }: { layer: Layer }) => (
  <ResultsCard label="Path" detail={getLayerDescription(layer)} isOk>
    <Box sx={{ mt: 3 }}>
      <NodePath layer={layer} />
    </Box>
  </ResultsCard>
);

export const Results = ({
  packetsSent,
  packetsReceived,
  layer,
}: {
  packetsSent: string;
  packetsReceived: string;
  layer: '1' | '2' | '3' | 'gateway';
}) => {
  const ref = useRef(null);
  const handleSaveToPdf = useReactToPrint({ documentTitle: 'Test results', content: () => ref.current });
  return (
    <>
      <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 1 }}>
        <Box display="flex" gap={1}>
          <Typography fontWeight="bold" component="span">
            Test date
          </Typography>
          <Typography>{format(new Date(), 'dd/MM/yyyy HH:mm')}</Typography>
        </Box>
        <Button onClick={handleSaveToPdf} startIcon={<Download />}>
          Save to PDF
        </Button>
      </Stack>
      <Grid container spacing={2} ref={ref}>
        <Grid item md={5}>
          <NodeSpeed Mbps={150.01} performance="good" />
        </Grid>
        <Grid item container direction="column" md={7}>
          <Stack spacing={2}>
            <Packets sent={packetsSent} received={packetsReceived} />
            <Path layer={layer} />
          </Stack>
        </Grid>
      </Grid>
    </>
  );
};
