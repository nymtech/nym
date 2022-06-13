import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { ConfirmTx } from './ConfirmTX';
import { ModalListItem } from './Modals/ModalListItem';

export default {
  title: 'Wallet / Confirm Transaction',
  component: ConfirmTx,
} as ComponentMeta<typeof ConfirmTx>;

const Template: ComponentStory<typeof ConfirmTx> = (args) => (
  <ConfirmTx {...args}>
    <ModalListItem label="Transaction type" value="Bond" divider />
    <ModalListItem label="Current bond" value="100 NYM" divider />
    <ModalListItem label="Additional bond" value="50 NYM" divider />
  </ConfirmTx>
);

export const Default = Template.bind({});
Default.args = {
  open: true,
  header: 'Confirm transaction',
  subheader: 'Confirm and proceed or cancel transaction',
  fee: { amount: { amount: '0.001', denom: 'NYM' } },
  onClose: () => {},
  onConfirm: async () => {},
  onPrev: () => {},
};
