import * as React from 'react';
import { useState } from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Button } from '@mui/material';
import { ConfirmationModal } from './ConfirmationModal';

export default {
  title: 'Modals/ConfirmationModal',
  component: ConfirmationModal,
} as ComponentMeta<typeof ConfirmationModal>;

const Template: ComponentStory<typeof ConfirmationModal> = (args) => {
  const [open, setOpen] = useState(true);
  return (
    <>
      <Button variant="outlined" onClick={() => setOpen(true)}>
        Open confirmation dialog
      </Button>
      <ConfirmationModal {...args} open={open} onClose={() => setOpen(false)} onConfirm={() => setOpen(false)}>
        Dialog content.
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
