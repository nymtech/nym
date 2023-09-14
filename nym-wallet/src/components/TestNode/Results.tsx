import React from 'react';
import { Grid } from '@mui/material';
import { TestStatus } from './types';
import { Packets } from './Packets';
import { NodeScore } from './NodeScore';
import { Overview } from './Overview';

export const Results = ({
  packetsSent = 0,
  packetsReceived = 0,
  score = 0,
  status,
  date = '-',
  onStartTest,
}: {
  packetsSent?: number;
  packetsReceived?: number;
  score?: number;
  status: TestStatus;
  date?: string;
  onStartTest: () => void;
}) => (
  <Grid container spacing={2} sx={{ mb: 3 }}>
    <Grid item xl={4} xs={12}>
      <Overview onStartTest={onStartTest} />
    </Grid>
    <Grid item xl={4} xs={6}>
      <NodeScore score={score} />
    </Grid>
    <Grid item xl={4} xs={6}>
      <Packets sent={packetsSent} received={packetsReceived} status={status} date={date} />
    </Grid>
  </Grid>
);
