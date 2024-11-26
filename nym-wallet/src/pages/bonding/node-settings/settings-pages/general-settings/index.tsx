import React, { useState } from 'react';
import { Box, Button, Divider, Grid } from '@mui/material';
import { isGateway, isMixnode, isNymNode } from 'src/types';
import { TBondedNode } from 'src/context/bonding';
import { GeneralMixnodeSettings } from './GeneralMixnodeSettings';
import { ParametersSettings } from './ParametersSettings';
import { GeneralGatewaySettings } from './GeneralGatewaySettings';
import { GeneralNymNodeSettings } from './GeneralNymNodeSettings';

const makeGeneralNav = (bondedNode: TBondedNode) => {
  const navItems = ['Info'];
  if (isMixnode(bondedNode)) {
    navItems.push('Parameters');
  }

  return navItems;
};

export const NodeGeneralSettings = ({ bondedNode }: { bondedNode: TBondedNode }) => {
  const [navSelection, setNavSelection] = useState<number>(0);

  const getSettings = () => {
    switch (navSelection) {
      case 0: {
        if (isMixnode(bondedNode)) return <GeneralMixnodeSettings bondedNode={bondedNode} />;
        if (isGateway(bondedNode)) return <GeneralGatewaySettings bondedNode={bondedNode} />;
        if (isNymNode(bondedNode)) return <GeneralNymNodeSettings bondedNode={bondedNode} />;
        break;
      }
      case 1: {
        if (isMixnode(bondedNode)) return <ParametersSettings bondedNode={bondedNode} />;
        if (isNymNode(bondedNode)) return null;
        break;
      }
      default:
        return null;
    }
    return undefined;
  };

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
        {getSettings()}
      </Grid>
    </Box>
  );
};
