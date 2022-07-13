import React, { useState } from 'react';
import { ErrorOutline } from '@mui/icons-material';
import { useTheme, Theme } from '@mui/material/styles';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Button } from '@mui/material';
import { ConfirmationModal } from './ConfirmationModal';
import { backDropStyles, dialogStyles } from '../../../.storybook/storiesStyles';

const storybookStyles = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: dialogStyles(theme),
});

export default {
  title: 'Modals/ConfirmationModal',
  component: ConfirmationModal,
} as ComponentMeta<typeof ConfirmationModal>;

const Template: ComponentStory<typeof ConfirmationModal> = (args) => {
  const [open, setOpen] = useState(true);
  const theme = useTheme();
  return (
    <>
      <Button variant="outlined" onClick={() => setOpen(true)}>
        Open confirmation dialog
      </Button>
      <ConfirmationModal
        {...args}
        open={open}
        onClose={() => setOpen(false)}
        onConfirm={() => setOpen(false)}
        {...storybookStyles(theme)}
      >
        Dialog content.
      </ConfirmationModal>
    </>
  );
};

export const withError: ComponentStory<typeof ConfirmationModal> = () => {
  const [open, setOpen] = useState(true);
  const theme = useTheme();
  return (
    <>
      <Button variant="outlined" onClick={() => setOpen(true)}>
        Open confirmation dialog
      </Button>
      <ConfirmationModal
        title="An error occured"
        confirmButton="Done"
        open={open}
        onClose={() => setOpen(false)}
        onConfirm={() => setOpen(false)}
        {...storybookStyles(theme)}
      >
        <ErrorOutline color="error" />
      </ConfirmationModal>
    </>
  );
};

export const Default = Template.bind({});
Default.args = {
  title: 'Confirmation Modal',
  subTitle: '',
  fullWidth: true,
  confirmButton: 'Confirm',
  maxWidth: 'xs',
  disabled: false,
};
