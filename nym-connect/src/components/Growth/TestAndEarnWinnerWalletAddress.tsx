import React from 'react';
import { Box, Button, Card, CardContent, CardMedia, Typography } from '@mui/material';
import { SxProps } from '@mui/system';
import Content from './content/TestAndEarn/WinnerEntersWalletAddress.mdx';

export const TestAndEarnWinnerWalletAddress: FCWithChildren<{
  sx?: SxProps;
}> = ({ sx }) => (
  <Box>
    <Content />
  </Box>
);
