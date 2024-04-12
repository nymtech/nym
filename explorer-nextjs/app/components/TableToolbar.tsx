'use client'

import React from 'react'
import { Box, SelectChangeEvent } from '@mui/material'
import { useIsMobile } from '@/app/hooks/useIsMobile'
import { Filters } from './Filters/Filters'

const fieldsHeight = '42.25px'

type TableToolBarProps = {
  childrenBefore?: React.ReactNode
  childrenAfter?: React.ReactNode
}

export const TableToolbar: FCWithChildren<TableToolBarProps> = ({
  childrenBefore,
  childrenAfter,
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
        {childrenAfter}
      </Box>
    </Box>
  )
}
