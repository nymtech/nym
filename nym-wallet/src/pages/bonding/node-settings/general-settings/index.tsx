import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Divider, Grid } from '@mui/material';
import { InfoSettings } from './InfoSettings';
import { ParametersSettings } from './ParametersSettings';
import { AppContext } from '../../../../context/main';

const nodeGeneralNav = ['Info', 'Parameters'];

export const NodeGeneralSettings = ({ onSaveChanges }: { onSaveChanges: () => void }) => {
  const [settingsCard, setSettingsCard] = useState<string>(nodeGeneralNav[0]);

  const { userBalance } = useContext(AppContext);

  useEffect(() => {
    console.log('a');
  }, [userBalance]);

  return (
    <Box sx={{ pl: 3, pt: 3 }}>
      <Grid container direction="row" spacing={3}>
        <Grid item container direction="column" width={0.2}>
          {nodeGeneralNav.map((item) => (
            <Button
              size="small"
              sx={{ p: 0, mr: 2, color: 'inherit', justifyContent: 'start' }}
              onClick={() => setSettingsCard(item)}
            >
              {item}
            </Button>
          ))}
        </Grid>
        <Divider orientation="vertical" flexItem />
        {settingsCard === nodeGeneralNav[0] && <InfoSettings onSaveChanges={() => console.log('saving...')} />}
        {settingsCard === nodeGeneralNav[1] && <h1>bye</h1>}
      </Grid>
    </Box>
  );
};
