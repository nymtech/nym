import React, { useRef } from 'react';
import { Download } from '@mui/icons-material';
import { Stack, Box, Typography, Button, Grid } from '@mui/material';
import { format } from 'date-fns';
import { useReactToPrint } from 'react-to-print';
import { TestStatus } from 'src/pages/bonding/node-settings/node-test/types';
import { Packets } from './Packets';
import { NodeScore } from './NodeScore';

export const Results = ({
  packetsSent = 0,
  packetsReceived = 0,
  score = 0,
  status,
}: {
  packetsSent?: number;
  packetsReceived?: number;
  score?: number;
  status: TestStatus;
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
          <NodeScore score={score} />
        </Grid>
        <Grid item container direction="column" md={7}>
          <Stack spacing={2}>
            <Packets sent={packetsSent} received={packetsReceived} score={score} status={status} />
          </Stack>
        </Grid>
      </Grid>
    </>
  );
};
