import React from 'react';
import { Avatar } from '@mui/material';
import stc from 'string-to-color';
import { TAccount } from 'src/types';

export const AccountAvatar = ({ name, address }: TAccount) => (
  <Avatar sx={{ bgcolor: stc(address), width: 35, height: 35, fontSize: 20 }}>{name.split('')[0]}</Avatar>
);
