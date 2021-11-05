import React from 'react';
import { Box, useMediaQuery, TextField, MenuItem } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import Select, { SelectChangeEvent } from '@mui/material/Select';

type TableToolBarProps = {
  onChangeSearch: (arg: string) => void;
  onChangePageSize: (event: SelectChangeEvent<string>) => void;
  pageSize: string;
  searchTerm: string;
};

export const TableToolbar: React.FC<TableToolBarProps> = ({
  searchTerm,
  onChangeSearch,
  onChangePageSize,
  pageSize,
}) => {
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));
  return (
    <>
      <Box
        sx={{
          width: '100%',
          marginBottom: 2,
          display: 'flex',
          flexDirection: matches ? 'column' : 'row',
          justifyContent: 'space-between',
        }}
      >
        <Select
          labelId="demo-simple-select-label"
          id="demo-simple-select"
          value={pageSize}
          onChange={onChangePageSize}
          sx={{
            width: 200,
            marginBottom: matches ? 2 : 0,
          }}
        >
          <MenuItem value={10}>10</MenuItem>
          <MenuItem value={30}>30</MenuItem>
          <MenuItem value={50}>50</MenuItem>
          <MenuItem value={100}>100</MenuItem>
        </Select>
        <TextField
          sx={{ width: 350 }}
          value={searchTerm}
          placeholder="search"
          onChange={(event) => onChangeSearch(event.target.value)}
        />
      </Box>
    </>
  );
};
