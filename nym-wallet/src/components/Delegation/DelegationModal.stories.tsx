import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Paper, Button } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
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

const backDropStyles = (theme: Theme) => {
  const { mode } = theme.palette;
  return {
    style: {
      left: mode === 'light' ? '0' : '50%',
      width: '50%',
    },
  };
};

const modalStyles = (theme: Theme) => {
  const { mode } = theme.palette;
  return { left: mode === 'light' ? '25%' : '75%' };
};

const Content: React.FC<{ children: React.ReactElement<any, any>; handleClick: () => void }> = ({
  children,
  handleClick,
}) => (
  <>
    <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
      <h2>Your Delegations</h2>
      <Button variant="contained" onClick={handleClick} sx={{ mb: 3 }}>
        Show modal story again
      </Button>
      <Delegations items={items} explorerUrl={explorerUrl} />
    </Paper>
    {children}
  </>
);

export const Loading = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="loading"
        action="delegate"
        BackdropProps={backDropStyles(theme)}
        sx={modalStyles(theme)}
      />
    </Content>
  );
};

export const DelegateSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="delegate"
        message="You delegated 5 NYM"
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};

export const UndelegateSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="undelegate"
        message="You undelegated 5 NYM"
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};

export const RedeemSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
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
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};

export const RedeemWithVestedSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
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
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};

export const RedeemAllSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
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
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};

export const Error = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  const { mode } = theme.palette;
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="error"
        action="redeem-all"
        message="Minim esse veniam Lorem id velit Lorem eu eu est. Excepteur labore sunt do proident proident sint aliquip consequat Lorem sint non nulla ad excepteur."
        recipient={recipient}
        balance={balance}
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        BackdropProps={{
          style: {
            left: mode === 'light' ? '0' : '50%',
            width: '50%',
          },
        }}
        sx={{
          left: mode === 'light' ? '25%' : '75%',
        }}
      />
    </Content>
  );
};
