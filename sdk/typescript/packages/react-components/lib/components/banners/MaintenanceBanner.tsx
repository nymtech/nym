import { Box, Collapse, Alert, IconButton, Typography, Divider } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import { SxProps } from '@mui/material';

export type BannerProps = {
  open: boolean;
  onClick: () => void;
  height?: number;
  sx?: SxProps;
};

export const MaintenanceBanner = (props: BannerProps) => {
  const { open, onClick, height, sx } = props;

  return (
    <Box sx={{ width: '100%', ...sx }}>
      <Collapse in={open}>
        <Alert
          id="maintenance-banner"
          action={
            <IconButton aria-label="close" color="inherit" size="small" onClick={onClick}>
              <CloseIcon fontSize="inherit" cursor="pointer" />
            </IconButton>
          }
          severity="success"
          icon={false}
          sx={{
            width: '100%',
            backgroundColor: (t) => t.palette.nym.highlight,
            borderRadius: 0,
            color: (t) => t.palette.nym.networkExplorer.nav.text,
            height: height || 'auto',
          }}
        >
          <Box display="flex">
            <Typography variant="body1" fontWeight={700}>
              NYM UPGRADE
            </Typography>
            <Divider orientation="vertical" flexItem sx={{ mx: '16px', borderRightWidth: 2 }} />
            <Typography variant="body2">
              The Nym mixnet smart contract upgrade has happened!{' '}
              <Box sx={{ fontWeight: 700 }} display="inline">
                Please make sure to upgrade your Nym services and apps to the latest version 1.1.0
              </Box>
            </Typography>
          </Box>
        </Alert>
      </Collapse>
    </Box>
  );
};
