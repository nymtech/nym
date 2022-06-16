import React from 'react';
import { Typography } from '@mui/material';
import { AppWindowFrame } from '../components/AppWindowFrame';
import { ConnectionButton } from '../components/ConnectionButton';
import { ConnectionStatusKind } from '../types';
import { NeedHelp } from '../components/NeedHelp';

export const DefaultLayout: React.FC<{
  status: ConnectionStatusKind;
  busy?: boolean;
  isError?: boolean;
  onConnectClick?: (status: ConnectionStatusKind) => void;
}> = ({ status, busy, isError, onConnectClick }) => (
  <AppWindowFrame>
    <Typography fontWeight="700" fontSize="14px" textAlign="center">
      Connect, your privacy will be 100% protected thanks to the Nym Mixnet
    </Typography>
    <Typography fontWeight="700" fontSize="14px" textAlign="center" color="#60D6EF" pt={2}>
      You are not protected now
    </Typography>
    <ConnectionButton status={status} busy={busy} isError={isError} onClick={onConnectClick} />
    <NeedHelp />
  </AppWindowFrame>
);
