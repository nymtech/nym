import React, { useState } from 'react';
import { Box, Button, Divider, Grid } from '@mui/material';
import { TBondedMixnode, TBondedGateway } from '../../../../../context/bonding';
import { InfoSettings } from './InfoSettings';
import { ParametersSettings } from './ParametersSettings';

const nodeGeneralNav = ['Info', 'Parameters'];

export const NodeGeneralSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const [settingsCard, setSettingsCard] = useState<string>(nodeGeneralNav[0]);
  //TODO: Check what happens with a gateway
  return (
    <Box sx={{ pl: 3, pt: 3 }}>
      <Grid container direction="row" spacing={3}>
        <Grid item container direction="column" xs={3}>
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
        {settingsCard === nodeGeneralNav[0] && <InfoSettings bondedNode={bondedNode} />}
        {settingsCard === nodeGeneralNav[1] && <ParametersSettings bondedNode={bondedNode} />}
      </Grid>
    </Box>
  );
};
