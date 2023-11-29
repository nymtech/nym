import * as React from 'react';
import { useClipboard } from 'use-clipboard-copy';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DoneIcon from '@mui/icons-material/Done';
import { Box, Tooltip } from '@mui/material';
import { SxProps } from '@mui/system';
import { DelegateSVG } from '../../../icons/DelevateSVG';
import { useChain } from '@cosmos-kit/react';

export const DelegateIconButton: FCWithChildren<{
  tooltip?: React.ReactNode;
  onDelegate: () => void;
  sx?: SxProps;
}> = ({ tooltip, onDelegate, sx }) => {
  const { address, getCosmWasmClient, isWalletConnected, getSigningCosmWasmClient } = useChain('nyx');
  console.log('isWalletConnected :>> ', isWalletConnected);

  const handleDelegateClick = () => {
    onDelegate();
  };
  return (
    <Tooltip title={isWalletConnected ? undefined : 'Connect your wallet to delegate'}>
      <Box sx={sx} onClick={isWalletConnected ? handleDelegateClick : undefined}>
        <DelegateSVG />
      </Box>
    </Tooltip>
  );
};
