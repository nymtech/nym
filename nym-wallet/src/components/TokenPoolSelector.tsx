import React, { useContext, useEffect, useState } from 'react'
import { FormControl, InputLabel, ListItemText, MenuItem, Select, SelectChangeEvent } from '@mui/material'
import { ClientContext } from '../context/main'

type TPoolOption = 'balance' | 'locked' | ''

export const TokenPoolSelector: React.FC<{ onSelect: (pool: TPoolOption) => void }> = ({ onSelect }) => {
  const [value, setValue] = useState<TPoolOption>('')
  const {
    userBalance: { tokenAllocation, balance },
    currency,
  } = useContext(ClientContext)

  useEffect(() => {
    if (value !== '') {
      onSelect(value)
    }
  }, [value])

  const handleChange = (e: SelectChangeEvent) => setValue(e.target.value as TPoolOption)

  return (
    <FormControl fullWidth>
      <InputLabel>Token pool</InputLabel>
      <Select label="Token Pool" onChange={handleChange} value={value}>
        <MenuItem value="balance">
          <ListItemText
            primary="Balance"
            secondary={`${balance?.printable_balance}`}
            secondaryTypographyProps={{ sx: { textTransform: 'uppercase' } }}
          />
        </MenuItem>
        <MenuItem value="locked">
          <ListItemText primary="Locked" secondary={`${tokenAllocation?.locked} ${currency?.major}`} />
        </MenuItem>
      </Select>
    </FormControl>
  )
}
