import React from 'react';
import { Card, CardContent, Divider, Stack, Typography } from '@mui/material';
import { SelectionChance } from '@nymproject/types';
import { InclusionProbability } from './InclusionProbability';

const computeSelectionProbability = (saturation: number): SelectionChance => {
  if (saturation < 5) return 'Low';

  if (saturation > 5 && saturation < 15) return 'Good';

  return 'High';
};

export const NodeDetails = ({ saturation }: { saturation?: string }) => {
  if (!saturation) return null;

  return (
    <Card variant="outlined" sx={{ p: 1 }}>
      <CardContent>
        <Stack direction="row" justifyContent="space-between">
          <Typography fontWeight="medium">Stake saturation</Typography>
          <Typography>{saturation || '- '}%</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Stack direction="row" justifyContent="space-between">
          <Typography fontWeight="medium">Selection probability</Typography>
          <InclusionProbability probability={computeSelectionProbability(parseInt(saturation, 10))} />
        </Stack>
      </CardContent>
    </Card>
  );
};
