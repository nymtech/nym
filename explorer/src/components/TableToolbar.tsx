import * as React from 'react';
import { Box, useMediaQuery, TextField, MenuItem } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import Select, { SelectChangeEvent } from '@mui/material/Select';

type TableToolBarProps = {
  onChangeSearch: (arg: string) => void;
  onChangePageSize: (event: SelectChangeEvent<string>) => void;
  pageSize: string;
  searchTerm: string;
  childrenBefore?: React.ReactNode;
  childrenAfter?: React.ReactNode;
};

export const TableToolbar: React.FC<TableToolBarProps> = ({
  searchTerm,
  onChangeSearch,
  onChangePageSize,
  pageSize,
  childrenBefore,
  childrenAfter,
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
          flexDirection: matches ? 'column-reverse' : 'row',
          justifyContent: 'space-between',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'middle' }}>
          {childrenBefore}
          <Select
            labelId="simple-select-label"
            id="simple-select"
            value={pageSize}
            onChange={onChangePageSize}
            sx={{
              width: matches ? 100 : 200,
            }}
          >
            <MenuItem value={10} data-testid="ten">
              10
            </MenuItem>
            <MenuItem value={30} data-testid="thirty">
              30
            </MenuItem>
            <MenuItem value={50} data-testid="fifty">
              50
            </MenuItem>
            <MenuItem value={100} data-testid="hundred">
              100
            </MenuItem>
          </Select>
        </Box>
        <Box sx={{ display: 'flex', alignItems: 'middle' }}>
          <TextField
            sx={{
              width: matches ? '100%' : 350,
              marginBottom: matches ? 2 : 0,
            }}
            value={searchTerm}
            data-testid="search-box"
            placeholder="search"
            onChange={(event) => onChangeSearch(event.target.value)}
          />
          {childrenAfter}
        </Box>
      </Box>
    </>
  );
};
