import React from 'react';
import { Box } from '@mui/material';
import { SxProps } from '@mui/system';
import Content from './content/TestAndEarn/WinnerEntersWalletAddress.mdx';

export const TestAndEarnWinnerWalletAddress: FCWithChildren<{
  sx?: SxProps;
}> = () => (
  <Box>
    <Content />
  </Box>
);
