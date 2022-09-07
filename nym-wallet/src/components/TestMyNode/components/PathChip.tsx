import React from 'react';
import { Chip } from '@mui/material';

export const PathChip = ({ label, highlight }: { label: string; highlight: boolean }) => (
  <Chip label={label} size="medium" color={highlight ? 'primary' : 'default'} />
);
