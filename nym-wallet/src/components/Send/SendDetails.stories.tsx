import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { useTheme, Theme } from '@mui/material/styles';
import { MockMainContextProvider } from 'src/context/mocks/main';
import { SendDetailsModal } from './SendDetailsModal';
import { SendSuccessModal } from './SendSuccessModal';
import { SendErrorModal } from './SendErrorModal';
import { SendInputModal } from './SendInputModal';
import { Send } from '.';
import { backDropStyles, modalStyles, dialogStyles } from '../../../.storybook/storiesStyles';

const storybookStylesModal = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: modalStyles(theme),
});

const storybookStylesDialog = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: dialogStyles(theme),
});

export default {
  title: 'Send/Components',
  component: SendDetailsModal,
} as ComponentMeta<typeof SendDetailsModal>;

export const SendInput = () => {
  const theme = useTheme();
  return (
    <SendInputModal
      toAddress=""
      fromAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
      denom="NYM"
      onNext={() => {}}
      onClose={() => {}}
      onAddressChange={() => {}}
      onAmountChange={() => {}}
      {...storybookStylesModal(theme)}
    />
  );
};

export const SendDetails = () => {
  const theme = useTheme();
  return (
    <SendDetailsModal
      fromAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
      toAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
      fee={{ amount: { amount: '0.01', denom: 'nym' }, fee: { Auto: null } }}
      denom="nym"
      amount={{ amount: '100', denom: 'nym' }}
      onPrev={() => {}}
      onSend={() => {}}
      onClose={() => {}}
      {...storybookStylesModal(theme)}
    />
  );
};

export const SendSuccess = () => {
  const theme = useTheme();
  return (
    <SendSuccessModal
      txDetails={{ amount: '100 NYM', txUrl: 'dummtUrl.com' }}
      onClose={() => {}}
      {...storybookStylesDialog(theme)}
    />
  );
};

export const SendError = () => {
  const theme = useTheme();
  return <SendErrorModal onClose={() => {}} {...storybookStylesModal(theme)} />;
};

export const SendFlow = () => {
  const theme = useTheme();
  return (
    <MockMainContextProvider>
      <Send hasStorybookStyles={{ backdropProps: { ...backDropStyles(theme) }, sx: modalStyles(theme) }} />
    </MockMainContextProvider>
  );
};
