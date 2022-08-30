import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, Grid } from '@mui/material';

type TSettingItem = {
  id: string;
  title: string;
  value: string;
};

type TDefaultSettings = {
  portSettings: TSettingItem[];
  hostSettings: TSettingItem[];
  versionSettings: TSettingItem[];
};

const defaultSettings: TDefaultSettings = {
  portSettings: [
    { id: 'mixPort', title: 'Mix port', value: '1789' },
    { id: 'verlocPort', title: 'Verloc Port', value: '1790' },
    { id: 'httpPort', title: 'HTTP Port', value: '8000' },
  ],
  hostSettings: [{ id: 'host', title: 'Host', value: '95.216.92.229' }],
  versionSettings: [{ id: 'version', title: 'Version', value: '95.216.92.229' }],
};

export const InfoSettings = ({ onSaveChanges }: { onSaveChanges: () => void }) => {
  const [valueChanged, setValueChanged] = useState<boolean>(false);
  const [settingsValue, setSettingsValue] = useState<TDefaultSettings>(defaultSettings);

  const handleValueChanged = (value: string, id: string) => {
    settingsValue?.portSettings?.map((item) => {
      console.log(item.id === id, defaultSettings);
      if (item.id === id) {
        const newItem = {
          id: item.id,
          title: item.title,
          value: value,
        };
        const updatedObject = {
          portSettings: newItem,
        };
        console.log('settingsValue', settingsValue, item);

        setSettingsValue((settingsValue) => ({
          ...settingsValue,
          ...updatedObject,
        }));
        // item.value = value;
      }
    });
    setValueChanged(true);
    console.log(value, id);
  };
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
            {settingsValue.portSettings.map((item) => (
              <Grid item width={1} spacing={3} key={item.id}>
                <TextField
                  type="input"
                  label={item.title}
                  value={item.value}
                  onChange={(e) => handleValueChanged(e.target.value, item.id)}
                  fullWidth
                />
              </Grid>
            ))}
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
            {settingsValue.hostSettings.map((item) => (
              <Grid item width={1} spacing={3} key={item.id}>
                <TextField
                  type="input"
                  label={item.title}
                  value={item.value}
                  onChange={(e) => handleValueChanged(e.target.value, item.id)}
                  fullWidth
                />
              </Grid>
            ))}
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
            {settingsValue.versionSettings.map((item) => (
              <Grid item width={1} spacing={3} key={item.id}>
                <TextField
                  type="input"
                  label={item.title}
                  value={item.value}
                  onChange={(e) => handleValueChanged(e.target.value, item.id)}
                  fullWidth
                />
              </Grid>
            ))}
          </Grid>
        </Grid>
        <Divider flexItem />
        <Grid container justifyContent="end">
          <Button
            size="large"
            variant="contained"
            disabled={!valueChanged}
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
