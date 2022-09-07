import React, { useState } from 'react';
import { NymCard } from 'src/components';
import { Overview } from 'src/components/TestMyNode/Overview';
import { Results } from 'src/components/TestMyNode/Results';
import { TestProgress } from 'src/components/TestMyNode/TestProgress';

export const TestNode = () => {
  const [view, setView] = useState('overview');
  const [packetsSent, setPacketsSent] = useState(0);
  const totalPackets = 500;

  const mockPacketTransfer = (sent: number) => {
    if (sent - 1 < totalPackets) {
      setPacketsSent(sent);
      setTimeout(() => {
        mockPacketTransfer(sent + 1);
      }, 12.5);
    }
    if (sent === totalPackets) {
      setView('results');
    }
  };

  const startTest = () => {
    setView('start-test');
    mockPacketTransfer(0);
  };

  return (
    <NymCard title="Test Node">
      {view === 'overview' && <Overview onStartTest={startTest} />}
      {view === 'start-test' && <TestProgress totalPackets={totalPackets} packetsSent={packetsSent} />}
      {view === 'results' && <Results />}
    </NymCard>
  );
};
