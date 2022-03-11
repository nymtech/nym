import { ListItem, ListItemText, Select } from '@mui/material';
import React, { useState } from 'react';

type TPool = 'balance' | 'locked';

export const TokenPoolSelector: React.FC = () => {
  const [value] = useState<TPool>();

  return (
    <Select label="Token Pool" value={value}>
      <ListItem>
        <ListItemText primary="Balance" secondary="123 nymt" />
      </ListItem>
    </Select>
  );
};
