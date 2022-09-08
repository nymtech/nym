import { useEffect, useState } from 'react';
import { Button, Divider, Typography, TextField, Grid, Alert } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { TBondedMixnode, TBondedGateway } from '../../../../context/bonding';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';

const getNumberlength = (number: number) => {
  return number.toString().length;
};

// TODO: adding ip regex that works well
const ipRegex = /^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/;
// TODO: only accept valid nym wallet versions
const appVersionRegex = /^\d+(?:\.\d+){2}$/gm;

export const InfoSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const { mixPort, verlocPort, httpApiPort, host, version } = bondedNode;

  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [mixPortUpdated, setMixPortUpdated] = useState<number>(mixPort);
  const [verlocPortUpdated, setVerlocPortUpdated] = useState<number>(verlocPort);
  const [httpApiPortUpdated, setHttpApiPortUpdated] = useState<number>(httpApiPort);
  const [hostUpdated, setHostUpdated] = useState<string>(host);
  const [versionUpdated, setVersionUpdated] = useState<string>(version);

  const theme = useTheme();

  useEffect(() => {
    setButtonActive(true);
    if (
      mixPortUpdated === mixPort &&
      verlocPortUpdated === verlocPort &&
      httpApiPortUpdated === httpApiPort &&
      hostUpdated === host &&
      versionUpdated === version
    ) {
      setButtonActive(false);
    }
    if (
      getNumberlength(mixPortUpdated) !== 4 ||
      getNumberlength(verlocPortUpdated) !== 4 ||
      getNumberlength(httpApiPortUpdated) !== 4 ||
      !versionUpdated.match(appVersionRegex)
    ) {
      setButtonActive(false);
    }
  }, [mixPortUpdated, verlocPortUpdated, httpApiPortUpdated, hostUpdated, versionUpdated]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { value, id } = e.target;
    const numNewValue = parseInt(value) || 0;

    switch (id) {
      case 'mixPort':
        setMixPortUpdated(numNewValue);
        break;
      case 'verlocPort':
        setVerlocPortUpdated(numNewValue);
        break;
      case 'httpApiPort':
        setHttpApiPortUpdated(numNewValue);
        break;
      case 'host':
        setHostUpdated(value);
        break;
      case 'version':
        setVersionUpdated(value);
    }
  };

  return (
    <Grid container xs>
      {buttonActive && (
        <Alert
          severity="info"
          sx={{
            px: 2,
            borderRadius: 0,
            bgcolor: 'background.default',
            color: (theme) => theme.palette.nym.nymWallet.text.blue,
            '& .MuiAlert-icon': { color: (theme) => theme.palette.nym.nymWallet.text.blue, mr: 1 },
          }}
        >
          <strong>Your changes will be ONLY saved on the display.</strong> Remember to change the values on your node’s
          config file too.
        </Alert>
      )}
      <Grid container>
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Port
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Change profit margin of your node
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                id="mixPort"
                type="input"
                label="Mix Port"
                value={mixPortUpdated}
                onChange={(e) => handleChange(e)}
                inputProps={{ maxLength: 4 }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                id="verlocPort"
                type="input"
                label="Verloc Port"
                value={verlocPortUpdated}
                onChange={(e) => handleChange(e)}
                inputProps={{ maxLength: 4 }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                id="httpApiPort"
                type="input"
                label="HTTP port"
                value={httpApiPortUpdated}
                onChange={(e) => handleChange(e)}
                inputProps={{ maxLength: 4 }}
                fullWidth
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Host
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock wallet after certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                id="host"
                type="input"
                label="host"
                value={hostUpdated}
                onChange={(e) => handleChange(e)}
                fullWidth
              />
            </Grid>
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Version
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Lock wallet after certain time
            </Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
              <TextField
                id="version"
                type="input"
                label="Version"
                value={versionUpdated}
                onChange={(e) => handleChange(e)}
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
            Save all display changes
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
          color: theme.palette.nym.nymWallet.text.blue,
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
    </Grid>
  );
};
