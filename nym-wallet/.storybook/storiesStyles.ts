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
