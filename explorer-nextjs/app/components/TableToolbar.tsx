'use client'

import React from 'react'
import {
  Box,
  TextField,
  MenuItem,
  FormControl,
  IconButton,
  Select,
  SelectChangeEvent,
} from '@mui/material'
import { Close } from '@mui/icons-material'
import { useIsMobile } from '@/app/hooks/useIsMobile'
import { Filters } from './Filters/Filters'

const fieldsHeight = '42.25px'

type TableToolBarProps = {
  onChangeSearch?: (arg: string) => void
  onChangePageSize: (event: SelectChangeEvent<string>) => void
  pageSize: string
  searchTerm?: string
  withFilters?: boolean
  childrenBefore?: React.ReactNode
  childrenAfter?: React.ReactNode
}

export const TableToolbar: FCWithChildren<TableToolBarProps> = ({
  searchTerm,
  childrenBefore,
  childrenAfter,
  withFilters,
  onChangeSearch,
}) => {
  const isMobile = useIsMobile()
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
      <Box
        sx={{
          display: 'flex',
          flexDirection: isMobile ? 'column-reverse' : 'row',
          alignItems: 'middle',
        }}
      >
        <Box
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            height: fieldsHeight,
          }}
        >
          {childrenBefore}
        </Box>
        {!!onChangeSearch && (
          <TextField
            sx={{
              width: isMobile ? '100%' : 200,
              marginBottom: isMobile ? 2 : 0,
            }}
            size="small"
            value={searchTerm}
            data-testid="search-box"
            placeholder="Search"
            InputProps={{
              endAdornment: searchTerm?.length ? (
                <IconButton size="small" onClick={() => onChangeSearch('')}>
                  <Close fontSize="small" />
                </IconButton>
              ) : undefined,
            }}
            onChange={(event) => onChangeSearch(event.target.value)}
          />
        )}
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
  )
}
