import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Box } from '@mui/material';
import { TestProgress } from '.';

export default {
  title: 'Test my node / Test progress',
  component: TestProgress,
} as ComponentMeta<typeof TestProgress>;

const Template: ComponentStory<typeof TestProgress> = ({ packetsSent, totalPackets }) => {
  const [sent, setSent] = React.useState(packetsSent);

  const mockPacketTransfer = (sent: number) => {
    if (sent - 1 < totalPackets) {
      setSent(sent);
      setTimeout(() => {
        mockPacketTransfer(sent + 1);
      }, 25);
    }
  };

  React.useEffect(() => {
    mockPacketTransfer(0);
  }, []);

  return (
    <Box display="flex" alignContent="center">
      <TestProgress packetsSent={sent} totalPackets={totalPackets} />
    </Box>
  );
};

export const Default = Template.bind({});
Default.args = {
  packetsSent: 0,
  totalPackets: 100,
};
