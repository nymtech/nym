import React from 'react';
import { Typography } from '@mui/material';

export const Title = ({ title, align = 'left' }: { title: string; align?: 'left' | 'center' }) => (
  <Typography
    component="h1"
    sx={{
      color: 'text.primary',
      fontWeight: 600,
      fontSize: { xs: '1.25rem', sm: '1.35rem' },
      lineHeight: 1.3,
      mb: 0.5,
      width: '100%',
      textAlign: align,
    }}
  >
    {title}
  </Typography>
);

export const Subtitle = ({ subtitle, align = 'left' }: { subtitle: string; align?: 'left' | 'center' }) => (
  <Typography
    sx={{
      color: 'text.secondary',
      textAlign: align,
      maxWidth: align === 'center' ? 520 : 'none',
      width: '100%',
    }}
  >
    {subtitle}
  </Typography>
);

export const SubtitleSlick = ({ subtitle }: { subtitle: string }) => (
  <Typography
    variant="caption"
    sx={{
      color: 'text.secondary',
      textTransform: 'uppercase',
      letterSpacing: 2,
      fontWeight: 500,
      fontSize: 12,
      display: 'block',
      width: '100%',
      mb: 2,
      textAlign: 'center',
    }}
  >
    {subtitle}
  </Typography>
);
