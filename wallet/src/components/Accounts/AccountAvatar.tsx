import { Avatar, Typography } from '@mui/material';
import stc from 'string-to-color';
import { TAccount } from '@src/types';

export const AccountAvatar = ({ name, small }: { name: TAccount['name']; small?: boolean }) => (
  <Avatar sx={{ bgcolor: stc(name), ...(small ? { width: 20, height: 20, pb: '1px' } : {}) }}>
    <Typography fontSize={small ? 14 : 'inherit'} fontWeight={600}>
      {name?.split('')[0]}
    </Typography>
  </Avatar>
);
