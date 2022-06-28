import * as React from 'react';
import { useState } from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { Button } from '@mui/material';
import SimpleDialog from './SimpleDialog';

export default {
  title: 'Bounding/SimpleDialog',
  component: SimpleDialog,
} as ComponentMeta<typeof SimpleDialog>;

const Template: ComponentStory<typeof SimpleDialog> = (args) => {
  const [open, setOpen] = useState(true);
  return (
    <>
      <Button variant="outlined" onClick={() => setOpen(true)}>
        Open simple dialog
      </Button>
      <SimpleDialog {...args} open={open} confirmButton="Confirm">
        Dialog content.
      </SimpleDialog>
    </>
  );
};

export const Default = Template.bind({});
Default.args = {
  title: 'Simple Dialog',
  subTitle: '',
  fullWidth: true,
  maxWidth: 'xs',
  closeButton: false,
  cancelButton: false,
  disabled: false,
};
