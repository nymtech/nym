import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Paper, Button } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { Delegations } from './Delegations';
import { items } from './DelegationList.stories';
import { DelegationModal } from './DelegationModal';
import { backDropStyles, modalStyles } from '../../../.storybook/storiesStyles';

const explorerUrl = 'https://sandbox-explorer.nymtech.net';

const storybookStyles = (theme: Theme) => ({
  backdropProps: backDropStyles(theme),
  sx: modalStyles(theme),
});

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

const Content: React.FC<{ children: React.ReactElement<any, any>; handleClick: () => void }> = ({
  children,
  handleClick,
}) => (
  <>
    <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
      <h2>Your Delegations</h2>
      <Button variant="contained" onClick={handleClick} sx={{ mb: 3 }}>
        Show modal
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
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const DelegateSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="delegate"
        message="You delegated 5 NYM"
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const UndelegateSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="undelegate"
        message="You undelegated 5 NYM"
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const RedeemSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="redeem"
        message="42 NYM"
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const RedeemWithVestedSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="redeem"
        message="42 NYM"
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const RedeemAllSuccess = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="success"
        action="redeem-all"
        message="42 NYM"
        transactions={
          theme.palette.mode === 'light'
            ? [transaction, transaction]
            : [transactionForDarkTheme, transactionForDarkTheme]
        }
        {...storybookStyles(theme)}
      />
    </Content>
  );
};

export const Error = () => {
  const [open, setOpen] = React.useState<boolean>(false);
  const handleClick = () => setOpen(true);
  const theme = useTheme();
  return (
    <Content handleClick={handleClick}>
      <DelegationModal
        open={open}
        onClose={() => setOpen(false)}
        status="error"
        action="redeem-all"
        message="Minim esse veniam Lorem id velit Lorem eu eu est. Excepteur labore sunt do proident proident sint aliquip consequat Lorem sint non nulla ad excepteur."
        transactions={theme.palette.mode === 'light' ? [transaction] : [transactionForDarkTheme]}
        {...storybookStyles(theme)}
      />
    </Content>
  );
};
