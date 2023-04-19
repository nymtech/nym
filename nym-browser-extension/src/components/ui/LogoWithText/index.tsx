import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Logo } from '../Logo';
import { Title } from '../Title';

export const LogoWithText = ({
  logoSmall,
  title,
  description,
}: {
  logoSmall?: boolean;
  title: string;
  description?: string;
}) => (
  <Stack alignItems="center" justifyContent="center" gap={3}>
    <Logo small={logoSmall} />
    <Title>{title}</Title>
    <Typography sx={{ color: 'grey.700', textAlign: 'center' }}>{description}</Typography>
  </Stack>
);
