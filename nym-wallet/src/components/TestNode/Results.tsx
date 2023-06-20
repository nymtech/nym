import React, { useRef } from 'react';
import { Download } from '@mui/icons-material';
import { Box, Button, Grid } from '@mui/material';

import { useReactToPrint } from 'react-to-print';
import { TestStatus } from 'src/pages/bonding/node-settings/node-test/types';
import { Packets } from './Packets';
import { NodeScore } from './NodeScore';
import { Overview } from './Overview';

export const Results = ({
  packetsSent = 0,
  packetsReceived = 0,
  score = 0,
  status,
  onStartTest,
}: {
  packetsSent?: number;
  packetsReceived?: number;
  score?: number;
  status: TestStatus;
  onStartTest: () => void;
}) => {
  const ref = useRef(null);
  const handleSaveToPdf = useReactToPrint({ documentTitle: 'Node test results', content: () => ref.current });

  return (
    <>
      <Grid container spacing={2} ref={ref} sx={{ mb: 3 }}>
        <Grid item xl={4} xs={12}>
          <Overview onStartTest={onStartTest} />
        </Grid>
        <Grid item xl={4} xs={6}>
          <NodeScore score={score} />
        </Grid>
        <Grid item xl={4} xs={6}>
          <Packets sent={packetsSent} received={packetsReceived} status={status} />
        </Grid>
      </Grid>
      <Box display="flex" justifyContent="flex-end">
        <Button onClick={handleSaveToPdf} startIcon={<Download />}>
          Save test results as PDF
        </Button>
      </Box>
    </>
  );
};
