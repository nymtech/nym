import React, { useContext, useEffect, useState } from 'react';
import { FormControl, InputLabel, ListItemText, MenuItem, Select, SelectChangeEvent, Typography } from '@mui/material';
import { AppContext } from '../context/main';

export type TPoolOption = 'balance' | 'locked';

export const TokenPoolSelector: React.FC<{ disabled: boolean; onSelect: (pool: TPoolOption) => void }> = ({
  disabled,
  onSelect,
}) => {
  const [value, setValue] = useState<TPoolOption>('balance');
  const {
    userBalance: { tokenAllocation, balance, fetchBalance, fetchTokenAllocation },
    clientDetails,
  } = useContext(AppContext);

  useEffect(() => {
    (async () => {
      await fetchBalance();
      await fetchTokenAllocation();
    })();
  }, []);

  useEffect(() => {
    onSelect(value);
  }, [value]);

  const handleChange = (e: SelectChangeEvent) => setValue(e.target.value as TPoolOption);

  return (
    <FormControl fullWidth>
      <InputLabel>Token pool</InputLabel>
      <Select
        label="Token Pool"
        onChange={handleChange}
        value={value}
        disabled={disabled}
        renderValue={(val) => <Typography sx={{ textTransform: 'capitalize' }}>{val}</Typography>}
      >
        <MenuItem value="balance">
          <ListItemText
            primary="Balance"
            secondary={`${balance?.printable_balance}`}
            secondaryTypographyProps={{ sx: { textTransform: 'uppercase' } }}
          />
        </MenuItem>
        <MenuItem value="locked">
          {tokenAllocation && (
            <ListItemText
              primary="Locked"
              secondary={`${+tokenAllocation.locked + +tokenAllocation.spendable} ${clientDetails?.mix_denom}`}
              sx={{ textTransform: 'uppercase' }}
            />
          )}
        </MenuItem>
      </Select>
    </FormControl>
  );
};
