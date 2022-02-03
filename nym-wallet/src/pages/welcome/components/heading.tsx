import React from 'react'
import { Typography } from '@mui/material'

export const Title = ({ title }: { title: string }) => (
  <Typography sx={{ color: 'common.white', fontWeight: 600 }}>{title}</Typography>
)

export const Subtitle = ({ subtitle }: { subtitle: string }) => (
  <Typography sx={{ color: 'common.white', textAlign: 'center', maxWidth: 400 }}>{subtitle}</Typography>
)

export const SubtitleSlick = ({ subtitle }: { subtitle: string }) => (
  <Typography variant="caption" sx={{ color: 'grey.600', textTransform: 'uppercase', letterSpacing: 4 }}>
    {subtitle}
  </Typography>
)
