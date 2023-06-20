import React from 'react';
import { Divider, Typography } from '@mui/material';
import { TestStatus } from 'src/pages/bonding/node-settings/node-test/types';
import { format } from 'date-fns';
import { ResultsCard, ResultsCardDetail } from './ResultsCard';

export const Packets = ({ sent, received, status }: { sent: number; received: number; status: TestStatus }) => (
  <ResultsCard label={<Typography fontWeight="bold">Status</Typography>} detail="">
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Test date" detail={format(new Date(), 'dd/MM/yyyy HH:mm')} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets sent" detail={sent.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets received" detail={received.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Test status" detail={status} />
  </ResultsCard>
);
