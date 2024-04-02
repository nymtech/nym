import React from 'react';
import { Box, Typography } from '@mui/material';
import { ElipsSVG } from '@src/icons/ElipsSVG';
import { trimAddress } from '@src/utils';
import { useWalletContext } from '@src/context/wallet';

export const WalletAddress = () => {
  const { address } = useWalletContext();

  const displayAddress = trimAddress(address, 7);

  return (
    <Box display="flex" alignItems="center" gap={0.5}>
      <ElipsSVG />
      <Typography variant="body1" fontWeight={600}>
        {displayAddress}
      </Typography>
    </Box>
  );
};
