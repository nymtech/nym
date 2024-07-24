import { Typography } from '@mui/material';
import { SelectionChance } from '@nymproject/types';

const colorMap: { [key in SelectionChance]: string } = {
  Low: 'error.main',
  Good: 'warning.main',
  High: 'success.main',
};

const textMap: { [key in SelectionChance]: string } = {
  Low: 'Low',
  Good: 'Good',
  High: 'High',
};

export const InclusionProbability = ({ probability }: { probability: SelectionChance }) => (
  <Typography sx={{ color: colorMap[probability] }}>{textMap[probability]}</Typography>
);
