import { useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, Grid } from '@mui/material';

type TSettingItem = {
  id: string;
  title: string;
  value: string;
};

const portRegex = /^\d{4}$/;
const ipRegex =
  /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
// TODO: only accept valid nym wallet versions
const appVersionRegex = /^\d+(?:\.\d+){2}$/gm;

const currentMixPort: TSettingItem = { id: 'mixPort', title: 'Mix port', value: '1789' };
const currentVerlocPort: TSettingItem = { id: 'verlocPort', title: 'Verloc Port', value: '1790' };
const currentHttpPort: TSettingItem = { id: 'httpPort', title: 'HTTP Port', value: '8000' };
const currentHost: TSettingItem = { id: 'host', title: 'Host', value: '95.216.92.229' };
const currentVersion: TSettingItem = { id: 'version', title: 'Version', value: '1.0.8' };

export const InfoSettings = ({ onSaveChanges }: { onSaveChanges: () => void }) => {
  const [valueChanged, setValueChanged] = useState<boolean>(false);
  const [buttonActive, setButtonActive] = useState<boolean>(false);
  const [mixPort, setMixPort] = useState<TSettingItem>(currentMixPort);
  const [verloc, setVerloc] = useState<TSettingItem>(currentVerlocPort);
  const [httpPort, setHttpPort] = useState<TSettingItem>(currentHttpPort);
  const [host, setHost] = useState<TSettingItem>(currentHost);
  const [version, setVersion] = useState<TSettingItem>(currentVersion);

  useEffect(() => {
    if (valueChanged) {
      if (
        Boolean(mixPort.value.match(portRegex)) &&
        Boolean(verloc.value.match(portRegex)) &&
        Boolean(httpPort.value.match(portRegex)) &&
        Boolean(host.value.match(ipRegex)) &&
        Boolean(version.value.match(appVersionRegex))
      ) {
        setButtonActive(true);
        return;
      }
    }
    setButtonActive(false);
  }, [valueChanged, mixPort, verloc, httpPort, host, version]);

  return (
    <Box sx={{ width: 0.78 }}>
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
                label={mixPort.title}
                value={mixPort.value}
                onChange={(e) => {
                  setMixPort({ ...mixPort, value: e.target.value });
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                type="input"
                label={verloc.title}
                value={verloc.value}
                onChange={(e) => {
                  setVerloc({ ...verloc, value: e.target.value });
                  setValueChanged(true);
                }}
                fullWidth
              />
            </Grid>
            <Grid item width={1}>
              <TextField
                type="input"
                label={httpPort.title}
                value={httpPort.value}
                onChange={(e) => {
                  setHttpPort({ ...httpPort, value: e.target.value });
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
                label={host.title}
                value={host.value}
                onChange={(e) => {
                  setHost({ ...host, value: e.target.value });
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
                label={version.title}
                value={version.value}
                onChange={(e) => {
                  setVersion({ ...version, value: e.target.value });
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
            onClick={onSaveChanges}
            sx={{ m: 3, width: '320px' }}
          >
            Save all changes
          </Button>
        </Grid>
      </Grid>
    </Box>
  );
};
