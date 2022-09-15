import * as React from 'react';
import { Box, TextField, MenuItem } from '@mui/material';
import Select, { SelectChangeEvent } from '@mui/material/Select';
import { Filters } from './Filters/Filters';
import { useIsMobile } from '../hooks/useIsMobile';

type TableToolBarProps = {
  onChangeSearch: (arg: string) => void;
  onChangePageSize: (event: SelectChangeEvent<string>) => void;
  pageSize: string;
  searchTerm: string;
  withFilters?: boolean;
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
  withFilters,
}) => {
  const isMobile = useIsMobile();
  return (
    <Box
      sx={{
        width: '100%',
        marginBottom: 2,
        display: 'flex',
        flexDirection: isMobile ? 'column' : 'row',
        justifyContent: 'space-between',
      }}
    >
      <Box sx={{ display: 'flex', flexDirection: isMobile ? 'column-reverse' : 'row', alignItems: 'middle' }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
          {childrenBefore}
          <Select
            labelId="simple-select-label"
            id="simple-select"
            value={pageSize}
            onChange={onChangePageSize}
            sx={{
              width: isMobile ? '50%' : 200,
              marginRight: isMobile ? 0 : 2,
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
        <TextField
          sx={{
            width: isMobile ? '100%' : 200,
            marginBottom: isMobile ? 2 : 0,
          }}
          value={searchTerm}
          data-testid="search-box"
          placeholder="search"
          onChange={(event) => onChangeSearch(event.target.value)}
        />
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'end',
          gap: 1,
          marginTop: isMobile ? 2 : 0,
        }}
      >
        {withFilters && <Filters />}
        {childrenAfter}
      </Box>
    </Box>
  );
};
