import React from 'react';
import { Divider } from '@mui/material';
import { ResultsCard, ResultsCardDetail } from './ResultsCard';
import { TestStatus } from 'src/pages/bonding/node-settings/node-test/types';

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
}) => {
  return (
    <ResultsCard label="Packets" detail={''} isOk={score > 75}>
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets sent" detail={sent.toString()} />
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets received" detail={received.toString()} />
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Test status" detail={status} />
    </ResultsCard>
  );
};
