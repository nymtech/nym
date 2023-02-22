import { Box, Stack } from '@mui/material';
import React from 'react';

export const ConnectionLayout = ({
  TopContent,
  ConnectButton,
  BottomContent,
}: {
  TopContent: React.ReactNode;
  ConnectButton: React.ReactNode;
  BottomContent: React.ReactNode;
}) => (
  <Stack direction="column" spacing={1} minHeight="100%" justifyContent="space-around" mt="-40px" mb="-40px">
    <Stack direction="column" height="24%" justifyContent="end">
      {TopContent}
    </Stack>
    <Box display="flex" justifyContent="center" alignItems="center">
      {ConnectButton}
    </Box>
    <Stack direction="column" justifySelf="flex-start" flexGrow={1}>
      {BottomContent}
    </Stack>
  </Stack>
);
