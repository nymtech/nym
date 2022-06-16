import React from 'react';
import { Box, SxProps } from '@mui/material';

export const ModalDivider: React.FC<{
  sx?: SxProps;
}> = ({ sx }) => <Box borderTop="1px solid" borderColor="rgba(141, 147, 153, 0.2)" my={1} sx={sx} />;
