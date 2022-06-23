import React from 'react';
import { ComponentMeta } from '@storybook/react';
import { MockMainContextProvider } from 'src/context/mocks/main';
import { SendDetailsModal } from './SendDetailsModal';
import { SendSuccessModal } from './SendSuccessModal';
import { SendErrorModal } from './SendErrorModal';
import { SendInputModal } from './SendInputModal';
import { Send } from '.';

export default {
  title: 'Send/Components',
  component: SendDetailsModal,
} as ComponentMeta<typeof SendDetailsModal>;

export const SendInput = () => (
  <SendInputModal
    toAddress=""
    fromAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
    onNext={() => {}}
    onClose={() => {}}
    onAddressChange={() => {}}
    onAmountChange={() => {}}
  />
);

export const SendDetails = () => (
  <SendDetailsModal
    fromAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
    toAddress="nymt1w8qp7zsxggvtxhpqpt6e329j42wtv07dm5ts8u"
    fee={{ amount: { amount: '0.01', denom: 'NYM' } }}
    amount={{ amount: '100', denom: 'NYM' }}
    onPrev={() => {}}
    onSend={() => {}}
    onClose={() => {}}
  />
);

export const SendSuccess = () => (
  <SendSuccessModal txDetails={{ amount: '100 NYM', txUrl: 'dummtUrl.com' }} onClose={() => {}} />
);

export const SendError = () => <SendErrorModal onClose={() => {}} />;

export const SendFlow = () => (
  <MockMainContextProvider>
    <Send />
  </MockMainContextProvider>
);
