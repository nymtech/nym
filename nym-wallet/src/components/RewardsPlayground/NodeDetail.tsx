import React from 'react';
import { Card, CardContent, Divider, Stack, Typography } from '@mui/material';
import { SelectionChance } from '@nymproject/types';
import { InclusionProbability } from './InclusionProbability';

export const NodeDetails = ({
  saturation,
  selectionProbability,
}: {
  saturation?: string;
  selectionProbability: SelectionChance;
}) => (
  <Card variant="outlined" sx={{ p: 1 }}>
    <CardContent>
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Stake saturation</Typography>
        <Typography>{saturation || '- '}%</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight="medium">Selection probability</Typography>
        <InclusionProbability probability={selectionProbability} />
      </Stack>
    </CardContent>
  </Card>
);
