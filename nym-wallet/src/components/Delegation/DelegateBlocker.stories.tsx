import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { useTheme } from '@mui/material/styles';
import { Button, Paper, Typography } from '@mui/material';

import { OverSaturatedBlockerModal } from './DelegateBlocker';

export default {
  title: 'Delegation/Components/Delegation Over Saturated Warning Modal',
  component: OverSaturatedBlockerModal,
} as ComponentMeta<typeof OverSaturatedBlockerModal>;

export const Default = () => {
  const [open, setOpen] = React.useState<boolean>(true);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <>
      <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
        <h2>Lorem ipsum</h2>
        <Button variant="contained" onClick={handleClick} sx={{ mb: 3 }}>
          Show modal
        </Button>
        <Typography>
          Veniam dolor laborum labore sit reprehenderit enim mollit magna nulla adipisicing fugiat. Est ex irure quis
          sunt velit elit do minim mollit non duis reprehenderit. Eiusmod dolore adipisicing ex nostrud consectetur
          culpa exercitation do. Ad elit esse ipsum aliqua labore irure laborum qui culpa.
        </Typography>
      </Paper>
      <OverSaturatedBlockerModal
        open={open}
        header="Node saturation: 114%"
        onClose={() => setOpen(false)}
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};
