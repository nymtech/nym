import React from 'react';
import { Button, Grid, Stack } from '@mui/material';
import testNode from 'src/assets/test-node-illustration.jpg';
import { DescriptionItem } from '../components/overview';

const content = [
  {
    title: 'How is works',
    description:
      'This is your APY playground - play with the parameters on left to see estimated rewards on the right side',
  },
  {
    title: 'Test path',
    description:
      'This is your APY playground - play with the parameters on left to see estimated rewards on the right side',
  },
  {
    title: 'Results',
    description:
      'This is your APY playground - play with the parameters on left to see estimated rewards on the right side',
  },
];

export const Overview = ({ onStartTest }: { onStartTest: () => void }) => (
  <Grid container spacing={3}>
    <Grid item md={12} lg={6}>
      <img src={testNode} />
    </Grid>
    <Grid item container direction="column" md={12} lg={6}>
      <Grid item>
        <Stack>{content.map(DescriptionItem)}</Stack>
      </Grid>
      <Grid item>
        <Button fullWidth variant="contained" disableElevation onClick={onStartTest}>
          Start test
        </Button>
      </Grid>
    </Grid>
  </Grid>
);
