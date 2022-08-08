import React from 'react';
import { Avatar, Typography } from '@mui/material';
import stc from 'string-to-color';
import { TAccount } from 'src/types';

export const AccountAvatar = ({ name, small }: { name: TAccount['name']; small?: boolean }) => (
  <Avatar sx={{ bgcolor: stc(name), ...(small ? { width: 25, height: 25 } : {}) }}>
    <Typography fontSize={small ? 14 : 'inherit'} fontWeight={600}>
      {name?.split('')[0]}
    </Typography>
  </Avatar>
);
