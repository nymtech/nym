import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Paper } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Delegations } from './Delegations';
import { items } from './DelegationList.stories';
import { DelegationModal } from './DelegationModal';

const explorerUrl = 'https://sandbox-explorer.nymtech.net';

export default {
  title: 'Delegation/Components/Delegation Modals',
  component: Delegations,
} as ComponentMeta<typeof Delegations>;

const transaction = {
  url: 'https://sandbox-blocks.nymtech.net/transactions/11ED7B9E21534A9421834F52FED5103DC6E982949C06335F5E12EFC71DAF0CFB',
  hash: '11ED7B9E21534A9421834F52FED5103DC6E982949C06335F5E12EFC71DAF0CFB',
};
// Another transaction for Dark Theme to avoid duplicate key errors in rendering
const transactionForDarkTheme = {
  url: 'https://sandbox-blocks.nymtech.net/transactions/11ED7B9E21534A9421834F52FED5103DC6E982949C06335F5E12EFC71DAF0CFO',
  hash: '11ED7B9E21534A9421834F52FED5103DC6E982949C06335F5E12EFC71DAF0CF0',
};
const balance = '104 NYMT';
const balanceVested = '12 NYMT';
const recipient = 'nymt1923pujepxfnv8dqyxqrl078s4ysf3xn2p7z2xa';

const Content: React.FC = () => (
  <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
    <h2>Your Delegations</h2>
    <Delegations items={items} explorerUrl={explorerUrl} />
  </Paper>
);

export const Loading = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="loading"
        action="delegate"
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const DelegateSuccess = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="success"
        action="delegate"
        message="You delegated 5 NYM"
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const UndelegateSuccess = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="success"
        action="undelegate"
        message="You undelegated 5 NYM"
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const RedeemSuccess = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="success"
        action="redeem"
        message="42 NYM"
        recipient={recipient}
        balance={balance}
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const RedeemWithVestedSuccess = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="success"
        action="redeem"
        message="42 NYM"
        recipient={recipient}
        balance={balance}
        balanceVested={balanceVested}
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const RedeemAllSuccess = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="success"
        action="redeem-all"
        message="42 NYM"
        recipient={recipient}
        balance={balance}
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};

export const Error = () => {
  const theme = useTheme();
  return (
    <>
      <Content />
      <DelegationModal
        status="error"
        action="redeem-all"
        message="Minim esse veniam Lorem id velit Lorem eu eu est. Excepteur labore sunt do proident proident sint aliquip consequat Lorem sint non nulla ad excepteur."
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        open
        sx={{ left: theme.palette.mode === 'light' ? '25%' : '75%' }}
      />
    </>
  );
};
