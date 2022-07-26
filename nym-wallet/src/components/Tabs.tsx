import React from 'react';
import { Tab, Tabs as MuiTabs } from '@mui/material';

export const Tabs: React.FC<{
  tabs: string[];
  selectedTab: number;
  disabled?: boolean;
  onChange?: (event: React.SyntheticEvent, tab: number) => void;
  disableActiveTabHighlight?: boolean;
}> = ({ tabs, selectedTab, disabled, disableActiveTabHighlight, onChange }) => (
  <MuiTabs
    value={selectedTab}
    onChange={onChange}
    sx={{
      bgcolor: (theme) => theme.palette.nym.nymWallet.background.grey,
      borderTop: '1px solid',
      borderBottom: '1px solid',
      borderColor: (theme) => theme.palette.nym.nymWallet.background.greyStroke,
    }}
    textColor="inherit"
    TabIndicatorProps={
      disableActiveTabHighlight
        ? {
            style: {
              opacity: 0,
            },
          }
        : {}
    }
  >
    {tabs.map((tabName) => (
      <Tab key={tabName} label={tabName} sx={{ textTransform: 'capitalize' }} disabled={disabled} />
    ))}
  </MuiTabs>
);
