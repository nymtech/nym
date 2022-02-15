import React from 'react'
import { InfoOutlined } from '@mui/icons-material'
import { Tooltip, TooltipProps } from '@mui/material'

export const InfoTooltip = ({
  title,
  tooltipPlacement = 'bottom',
  light,
}: {
  title: string
  tooltipPlacement?: TooltipProps['placement']
  light?: boolean
}) => (
  <Tooltip title={title} arrow placement={tooltipPlacement}>
    <InfoOutlined fontSize="small" sx={{ color: light ? 'grey.500' : undefined }} />
  </Tooltip>
)
