import { Box } from '@mui/material';
import React from 'react';

const layout = {
  display: 'grid',
  gridTemplateColumns: '1fr',
  gridTemplateRows: '80px 180px 1fr',
  gridColumnGap: '0px',
  gridRowGap: '4px',
  overflow: 'hidden',
  height: '100%',
};

export const ConnectionLayout = ({
  TopContent,
  ConnectButton,
  BottomContent,
}: {
  TopContent: React.ReactNode;
  ConnectButton: React.ReactNode;
  BottomContent: React.ReactNode;
}) => (
  <Box sx={layout}>
    {TopContent}
    <Box display="flex" justifyContent="center" alignItems="center">
      {ConnectButton}
    </Box>
    {BottomContent}
  </Box>
);
