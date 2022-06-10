import React from 'react';
import { Typography } from '@mui/material';
import { CircleOutlined, PauseCircleOutlined, CheckCircleOutline } from '@mui/icons-material';
import { MixnodeStatus } from '@nymproject/types';

const Active = () => (
  <Typography sx={{ color: 'success.main', display: 'flex', alignItems: 'center' }}>
    <CheckCircleOutline fontSize="small" color="success" sx={{ mr: 1 }} /> Active
  </Typography>
);

const Inactive = () => (
  <Typography sx={{ color: 'nym.text.dark', display: 'flex', alignItems: 'center' }}>
    <CircleOutlined fontSize="small" sx={{ color: 'nym.text.dark', mr: 1 }} /> Inactive
  </Typography>
);

const Standby = () => (
  <Typography sx={{ color: 'info.main', display: 'flex', alignItems: 'center' }}>
    <PauseCircleOutlined fontSize="small" color="info" sx={{ mr: 1 }} /> Standby
  </Typography>
);

export const NodeStatus = ({ status }: { status: MixnodeStatus }) => {
  switch (status) {
    case 'active':
      return <Active />;
    case 'inactive':
      return <Inactive />;
    case 'standby':
      return <Standby />;
    default:
      return null;
  }
  return null;
};
