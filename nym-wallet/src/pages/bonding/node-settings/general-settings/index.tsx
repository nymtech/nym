import React, { useContext, useEffect, useState } from 'react';
import { Box, Button, Divider, Grid } from '@mui/material';
import { InfoSettings } from './InfoSettings';
import { ParametersSettings } from './ParametersSettings';

const nodeGeneralNav = ['Info', 'Parameters'];

export const NodeGeneralSettings = ({ onSaveChanges }: { onSaveChanges: () => void }) => {
  const [settingsCard, setSettingsCard] = useState<string>(nodeGeneralNav[0]);

  return (
    <Box sx={{ pl: 3, pt: 3 }}>
      <Grid container direction="row" spacing={3}>
        <Grid item container direction="column" width={0.2}>
          {nodeGeneralNav.map((item) => (
            <Button
              size="small"
              sx={{
                fontSize: 14,
                color: settingsCard === item ? 'primary.main' : 'inherit',
                justifyContent: 'start',
                ':hover': {
                  bgcolor: 'transparent',
                  color: 'primary.main',
                },
              }}
              key={item}
              onClick={() => setSettingsCard(item)}
            >
              {item}
            </Button>
          ))}
        </Grid>
        <Divider orientation="vertical" flexItem />
        {settingsCard === nodeGeneralNav[0] && <InfoSettings onSaveChanges={() => console.log('saving...')} />}
        {settingsCard === nodeGeneralNav[1] && <ParametersSettings onSaveChanges={() => console.log('saving...')} />}
      </Grid>
    </Box>
  );
};
