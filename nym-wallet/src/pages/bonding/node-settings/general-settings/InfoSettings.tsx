import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Divider, Typography, TextField, Grid } from '@mui/material';

const ModifiedTextField = ({ field }: { field: { id: string; title: string; value: string } }) => {
  return (
    <Box>
      <Typography>{field.title}</Typography>
      <TextField
        type="input"
        value={field.value}
        onChange={(e) => console.log(`Field ${field.id} has change`, e.target.value)}
        autoFocus
        fullWidth
      />
    </Box>
  );
};

export const InfoSettings = ({ onSaveChanges }: { onSaveChanges: () => void }) => {
  const [valueChanged, setValueChanged] = useState<boolean>(false);
  const [mixPortValue, setMixPortValue] = useState<string>('1789');

  useEffect(() => {
    console.log(Object.entries(portSettings));
  }, []);
  const portSettings = [
    { id: 'mixPort', title: 'Mix port', value: '1789' },
    { id: 'verlocPort', title: 'Verloc Port', value: '1790' },
    { id: 'httpPort', title: 'HTTP Port', value: '8000' },
  ];

  const hostSettings = [{ id: 'host', title: 'Host', value: '95.216.92.229' }];
  const versionSettings = [{ id: 'version', title: 'Version', value: '95.216.92.229' }];
  return (
    <Box sx={{ width: 0.78 }}>
      <Grid container direction="column">
        <Grid
          item
          container
          direction="row"
          alignItems="left"
          justifyContent="space-between"
          flexWrap="nowrap"
          padding={3}
        >
          <Grid item direction="column">
            <Typography>Port</Typography>
            <Typography>Change profit margin of your node</Typography>
          </Grid>
          <Grid spacing={3} item container alignItems="center" maxWidth="320px">
            {portSettings.map((item) => (
              <Grid item width={1} spacing={3}>
                <TextField
                  type="input"
                  label={item.title}
                  value={item.value}
                  onChange={(e) => console.log(`Field ${item.id} has change`, e.target.value)}
                  autoFocus
                  fullWidth
                />
              </Grid>
            ))}
          </Grid>
        </Grid>
        <Divider flexItem />
        <Box display="flex">
          <Box display="flex" flexDirection="column">
            <Typography>Host</Typography>
            <Typography>Lock wallet after certain time</Typography>
          </Box>
          <Box>
            {hostSettings.map((item) => (
              <ModifiedTextField field={item} />
            ))}
          </Box>
        </Box>
        <Divider flexItem />
        <Box display="flex">
          <Box display="flex" flexDirection="column">
            <Typography>Version</Typography>
            <Typography>Lock wallet after certain time</Typography>
          </Box>
          <Box>
            {versionSettings.map((item) => (
              <ModifiedTextField field={item} />
            ))}
          </Box>
        </Box>
        <Divider flexItem />
        <Button variant="contained" disabled={!valueChanged} onClick={onSaveChanges}>
          Save all changes
        </Button>
      </Grid>
    </Box>
  );
};
