import Checkbox from '@mui/material/Checkbox';

const label = { inputProps: { 'aria-label': 'Checkbox demo' } };

export const PlaygroundCheckboxes: FCWithChildren = () => (
  <div>
    <Checkbox {...label} defaultChecked />
    <Checkbox {...label} />
    <Checkbox {...label} disabled />
    <Checkbox {...label} disabled checked />
  </div>
);
