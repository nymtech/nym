import React from 'react';
import { useMediaQuery, useTheme } from '@mui/material';
import { Box } from '@mui/system';
import { TextField, MenuItem } from '@mui/material';
import Select, { SelectChangeEvent } from '@mui/material/Select';


type TableToolBarProps = {
    onChangeSearch: (event: React.ChangeEvent<HTMLInputElement>) => void
    onChangePageSize: (event: SelectChangeEvent<string>) => void
    pageSize: string
}

export const TableToolbar = ({ onChangeSearch, onChangePageSize, pageSize }: TableToolBarProps) => {
    const theme = useTheme();
    const matches = useMediaQuery(theme.breakpoints.down("sm"));
    return (
        <>
            <Box
                sx={{
                    width: '100%',
                    marginBottom: 2,
                    display: 'flex',
                    flexDirection: matches ? 'column' : 'row',
                    justifyContent: 'space-between'
                }}>
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
                    placeholder="search"
                    onChange={onChangeSearch}
                />
            </Box>
        </>
    );
};
