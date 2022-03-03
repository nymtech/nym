import React from 'react'
import { InfoOutlined } from '@mui/icons-material'
import { Tooltip, TooltipProps } from '@mui/material'

export const InfoTooltip = ({
  title,
  tooltipPlacement = 'bottom',
  light,
  size = 'small',
}: {
  title: string
  tooltipPlacement?: TooltipProps['placement']
  light?: boolean
  size?: 'small' | 'medium' | 'large'
}) => (
  <Tooltip title={title} arrow placement={tooltipPlacement}>
    <InfoOutlined fontSize={size} sx={{ color: light ? 'grey.500' : undefined }} />
  </Tooltip>
)
