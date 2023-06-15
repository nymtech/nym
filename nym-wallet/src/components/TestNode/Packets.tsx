import React from 'react';
import { Divider } from '@mui/material';
import { ResultsCard, ResultsCardDetail } from './ResultsCard';

export const Packets = ({ sent, received, score }: { sent: number; received: number; score: number }) => {
  return (
    <ResultsCard label="Packets" detail={`${score.toString()}% packets`} isOk={score > 75}>
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets sent" detail={sent.toString()} />
      <Divider sx={{ my: 2 }} />
      <ResultsCardDetail label="Packets received" detail={received.toString()} />
    </ResultsCard>
  );
};
