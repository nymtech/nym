import React, { useState } from 'react';
import { Box, Button, Divider, Grid } from '@mui/material';
import { isGateway, isMixnode } from 'src/types';
import { TBondedMixnode, TBondedGateway } from 'src/context/bonding';
import { GeneralMixnodeSettings } from './GeneralMixnodeSettings';
import { ParametersSettings } from './ParametersSettings';
import { GeneralGatewaySettings } from './GeneralGatewaySettings';

const makeGeneralNav = (bondedNode: TBondedMixnode | TBondedGateway) => {
  const navItems = ['Info'];
  if (isMixnode(bondedNode)) {
    navItems.push('Parameters');
  }

  return navItems;
};

export const NodeGeneralSettings = ({ bondedNode }: { bondedNode: TBondedMixnode | TBondedGateway }) => {
  const [navSelection, setNavSelection] = useState<number>(0);

  return (
    <Box sx={{ pl: 3, pt: 3 }}>
      <Grid container direction="row" spacing={3}>
        <Grid item container direction="column" xs={3}>
          {makeGeneralNav(bondedNode).map((item, index) => (
            <Button
              size="small"
              sx={{
                fontSize: 14,
                color: navSelection === index ? 'primary.main' : 'inherit',
                justifyContent: 'start',
                ':hover': {
                  bgcolor: 'transparent',
                  color: 'primary.main',
                },
              }}
              key={item}
              onClick={() => setNavSelection(index)}
            >
              {item}
            </Button>
          ))}
        </Grid>
        <Divider orientation="vertical" flexItem />
        {navSelection === 0 && isMixnode(bondedNode) ? (
          <GeneralMixnodeSettings bondedNode={bondedNode} />
        ) : navSelection === 0 && isGateway(bondedNode) ? (
          <GeneralGatewaySettings bondedNode={bondedNode} />
        ) : null}
        {navSelection === 1 && isMixnode(bondedNode) && <ParametersSettings bondedNode={bondedNode} />}
      </Grid>
    </Box>
  );
};
