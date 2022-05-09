import React from 'react';
import { Avatar } from '@mui/material';
import stc from 'string-to-color';
import { TAccount } from 'src/types';

export const AccountAvatar = ({ name }: Pick<TAccount, 'name'>) => (
  <Avatar sx={{ bgcolor: stc(name), width: 35, height: 35 }}>{name?.split('')[0]}</Avatar>
);
