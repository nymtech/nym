import { useEffect } from 'react';
import { Card, CardContent, CardHeader, Dialog, Divider } from '@mui/material';

import { sleep } from '@src/utils/sleep';
import { ResultsCardDetail } from './ResultsCard';

export const PrintResults = ({
  packetsSent,
  packetsReceived,
  score,
  mixnodeId,
  mixnodeName,
  date = '',
  OnPrintRequestComplete,
}: {
  packetsSent: number;
  packetsReceived: number;
  score: number;
  mixnodeId: string;
  mixnodeName: string;
  date?: string;
  OnPrintRequestComplete: () => void;
}) => {
  const asyncPrint = async () => {
    await sleep(250);
    window.print();
  };

  useEffect(() => {
    asyncPrint();
    window.addEventListener('afterprint', OnPrintRequestComplete);

    return () => window.removeEventListener('afterprint', OnPrintRequestComplete);
  }, []);

  return (
    <Dialog fullScreen open onClick={OnPrintRequestComplete}>
      <Card sx={{ width: '100vw', height: '100vh', p: 3 }}>
        <CardHeader title="Node test results" action={<NymLogo />} sx={{ maxWidth: 800 }} />
        <CardContent sx={{ maxWidth: 800 }}>
          <ResultsCardDetail label="Date" detail={date} largeText />
          <Divider sx={{ my: 2 }} />
          <ResultsCardDetail label="Mixnode ID" detail={mixnodeId} largeText />
          <Divider sx={{ my: 2 }} />
          <ResultsCardDetail label="Mixnode name" detail={mixnodeName} largeText />
          <Divider sx={{ my: 2 }} />
          <ResultsCardDetail label="Packets sent" detail={packetsSent.toString()} largeText />
          <Divider sx={{ my: 2 }} />
          <ResultsCardDetail label="Packets received" detail={packetsReceived.toString()} largeText />
          <Divider sx={{ my: 2 }} />
          <ResultsCardDetail label="Performance score" detail={`${score.toString()}%`} largeText />
          <Divider sx={{ my: 2 }} />
        </CardContent>
      </Card>
    </Dialog>
  );
};
