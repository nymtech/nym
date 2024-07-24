export const modalStyle = {
  position: 'absolute' as const,
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: 600,
  bgcolor: 'background.paper',
  boxShadow: 24,
  borderRadius: '16px',
  p: 4,
};

import { Theme } from '@mui/material/styles';

export const backDropStyles = (theme: Theme) => {
  const { mode } = theme.palette;
  return {
    style: {
      left: mode === 'light' ? '0' : '50%',
      width: '50%',
    },
  };
};

export const modalStyles = (theme: Theme) => {
  const { mode } = theme.palette;
  return { left: mode === 'light' ? '25%' : '75%' };
};

export const dialogStyles = (theme: Theme) => {
  const { mode } = theme.palette;
  return { left: mode === 'light' ? '-50%' : '50%' };
};
