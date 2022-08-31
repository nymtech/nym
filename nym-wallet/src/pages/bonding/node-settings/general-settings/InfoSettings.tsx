import { useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, Grid, Alert } from '@mui/material';
import { TBondedMixnode, TBondedGateway } from '../../../../context/bonding';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

// TODO: adding ip regex that works well
const ipRegex = /^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/;
// TODO: only accept valid nym wallet versions
const appVersionRegex = /^\d+(?:\.\d+){2}$/gm;

export const InfoSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const { mixPort, verlocPort, httpApiPort, host, version } = bondedNode;

  const [valueChanged, setValueChanged] = useState<boolean>(false);
  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [mixPortUpdated, setMixPortUpdated] = useState<number>(mixPort);
  const [verlocPortUpdated, setVerlocPortUpdated] = useState<number>(verlocPort);
  const [httpApiPortUpdated, setHttpApiPortUpdated] = useState<number>(httpApiPort);
  const [hostUpdated, setHostUpdated] = useState<string>(host);
  const [versionUpdated, setVersionUpdated] = useState<string>(version);

  useEffect(() => {
    if (valueChanged) {
      if (
        Boolean(mixPortUpdated.toString().length === 4) &&
        Boolean(verlocPortUpdated.toString().length === 4) &&
        Boolean(httpApiPortUpdated.toString().length === 4) &&
        Boolean(versionUpdated.match(appVersionRegex))
      ) {
        setButtonActive(true);
        return;
      }
    }
    setButtonActive(false);
  }, [valueChanged, mixPortUpdated, verlocPortUpdated, httpApiPortUpdated, hostUpdated, versionUpdated]);

  return (
    <Box sx={{ width: '79.88%' }}>
      {buttonActive && (
        <Alert
          severity="info"
          sx={{
            px: 2,
            borderRadius: 0,
            bgcolor: 'background.default',
            color: 'info.dark',
            '& .MuiAlert-icon': { color: 'info.dark' },
          }}
        >
          <strong>Your changes will be ONLY saved on the display.</strong> Remember to change the values on your node’s
          config file too.
        </Alert>
      )}
      <Grid container>
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography sx={{ fontSize: 16, fontWeight: 600, mb: 1 }}>Port</Typography>
            <Typography
              sx={{
                fontSize: 14,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Change profit margin of your node
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="348px">
            <Grid item width={1}>
              <TextField
                type="input"
                label="Mix Port"
                value={mixPortUpdated}
                onChange={(e) => {
                  setMixPortUpdated(parseInt(e.target.value));
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                type="input"
                label="Verloc Port"
                value={verlocPortUpdated}
                onChange={(e) => {
                  setVerlocPortUpdated(parseInt(e.target.value));
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                type="input"
                label="HTTP port"
                value={httpApiPortUpdated}
                onChange={(e) => {
                  setHttpApiPortUpdated(parseInt(e.target.value));
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography sx={{ fontSize: 16, fontWeight: 600, mb: 1 }}>Host</Typography>
            <Typography
              sx={{
                fontSize: 14,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock wallet after certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="348px">
            <Grid item width={1}>
              <TextField
                type="input"
                label="host"
                value={hostUpdated}
                onChange={(e) => {
                  setHostUpdated(e.target.value);
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography sx={{ fontSize: 16, fontWeight: 600, mb: 1 }}>Version</Typography>
            <Typography
              sx={{
                fontSize: 14,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock wallet after certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="348px">
            <Grid item width={1}>
              <TextField
                type="input"
                label="Version"
                value={versionUpdated}
                onChange={(e) => {
                  setVersionUpdated(e.target.value);
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid container justifyContent="end">
          <Button
            size="large"
            variant="contained"
            disabled={!buttonActive}
            onClick={() => setOpenConfirmationModal(true)}
            sx={{ m: 3, width: '320px' }}
          >
            Save all changes
          </Button>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes were ONLY saved on the display"
        subHeader="Remember to change the values 
        on your node’s config file too."
        okLabel="close"
        hideCloseIcon
        displayInfoIcon
        onOk={async () => {
          await setOpenConfirmationModal(false);
        }}
        buttonFullWidth
        sx={{
          width: '450px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
        }}
        headerStyles={{
          width: '100%',
          mb: 1,
          textAlign: 'center',
          color: 'info.dark',
          fontSize: 16,
          textTransform: 'capitalize',
        }}
        subHeaderStyles={{
          width: '100%',
          mb: 1,
          textAlign: 'center',
          color: 'main',
          fontSize: 14,
          textTransform: 'capitalize',
        }}
      />
    </Box>
  );
};
