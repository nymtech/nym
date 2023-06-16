import React from 'react';
import { Divider } from '@mui/material';
import { TestStatus } from 'src/pages/bonding/node-settings/node-test/types';
import { ResultsCard, ResultsCardDetail } from './ResultsCard';

export const Packets = ({
  sent,
  received,
  score,
  status,
}: {
  sent: number;
  received: number;
  score: number;
  status: TestStatus;
}) => (
  <ResultsCard label="Packets" detail="" isOk={score > 75}>
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets sent" detail={sent.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets received" detail={received.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Test status" detail={status} />
  </ResultsCard>
);
