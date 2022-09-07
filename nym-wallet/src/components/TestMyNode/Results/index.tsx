import React from 'react';
import { ArrowForward, CheckCircleOutline, Description, Download } from '@mui/icons-material';
import { Box, Button, Card, Chip, CircularProgress, Divider, Grid, Stack, Typography } from '@mui/material';
import format from 'date-fns/format';
import { ResultsCard } from '../components/ResultsCard';
import { ResultsCardDetail } from '../components/ResultsCardDetail';
import { PathChip } from '../components/PathChip';

const NodeSpeed = ({ Mbps, performance }: { Mbps: number; performance: 'poor' | 'fair' | 'good' }) => (
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

const Packets = ({ sent, received }: { sent: string; received: string }) => (
  <ResultsCard label="Packets" detail="98% packets" isOk>
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets sent" detail={sent} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets received" detail={received} />
  </ResultsCard>
);

const Path = ({ layer }: { layer: number }) => (
  <ResultsCard label="Path" detail="Your node was in layer 2" isOk>
    <Stack direction="row" alignItems="center" gap={1} sx={{ mt: 2 }}>
      <PathChip label="Gateway" highlight={false} />
      <ArrowForward sx={{ color: 'grey.500' }} />
      <PathChip label="Node - Layer 1" highlight={layer === 1} />
      <ArrowForward sx={{ color: 'grey.500' }} />
      <PathChip label="Node - Layer 2" highlight={layer === 2} />
      <ArrowForward sx={{ color: 'grey.500' }} />
      <PathChip label="Node - Layer 3" highlight={layer === 3} />
    </Stack>
  </ResultsCard>
);

export const Results = () => (
  <>
    <Stack direction="row" justifyContent="space-between" alignItems="center" sx={{ mb: 1 }}>
      <Box display="flex" gap={1}>
        <Typography fontWeight="bold" component="span">
          Test date
        </Typography>
        <Typography>{format(new Date(), 'dd/MM/yyyy HH:mm')}</Typography>
      </Box>
      <Button startIcon={<Download />}>Save to PDF</Button>
    </Stack>
    <Grid container spacing={2}>
      <Grid item md={5}>
        <NodeSpeed Mbps={150.01} performance="good" />
      </Grid>
      <Grid item container direction="column" md={7}>
        <Stack spacing={2}>
          <Packets sent="5000" received="1000" />
          <Path layer={2} />
        </Stack>
      </Grid>
    </Grid>
  </>
);
