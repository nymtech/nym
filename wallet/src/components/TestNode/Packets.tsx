import { Divider, Typography } from '@mui/material';
import { TestStatus } from './types';
import { ResultsCard, ResultsCardDetail } from './ResultsCard';

export const Packets = ({
  sent,
  received,
  status,
  date,
}: {
  sent: number;
  received: number;
  status: TestStatus;
  date: string;
}) => (
  <ResultsCard label={<Typography fontWeight="bold">Status</Typography>} detail="">
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Test date" detail={date} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets sent" detail={sent.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Packets received" detail={received.toString()} />
    <Divider sx={{ my: 2 }} />
    <ResultsCardDetail label="Test status" detail={status} />
  </ResultsCard>
);
