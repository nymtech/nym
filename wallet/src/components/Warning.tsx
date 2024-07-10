import Alert, { AlertProps } from '@mui/material/Alert';
import { alpha, styled } from '@mui/material/styles';

const StyledAlert = styled(Alert)<AlertProps>(({ theme }) => ({
  backgroundColor: `${alpha(theme.palette.nym.nymWallet.background.warn, 0.1)}`,
  color: theme.palette.mode === 'light' ? theme.palette.nym.nymWallet.text.main : theme.palette.nym.nymWallet.text.warn,
  display: 'block',
  borderColor: theme.palette.nym.nymWallet.text.warn,
}));

export const Warning = ({ children, ...props }: { children?: React.ReactNode } & AlertProps) => (
  <StyledAlert icon={false} variant="outlined" severity="warning" {...props}>
    {children}
  </StyledAlert>
);
