import React from 'react';
import { Tab, Tabs as MuiTabs, SxProps } from '@mui/material';

type Props = {
  tabs: readonly string[];
  selectedTab: string;
  disabled?: boolean;
  onChange?: (event: React.SyntheticEvent, tab: string) => void;
  disableActiveTabHighlight?: boolean;
  tabSx?: SxProps;
  tabIndicatorStyles?: {};
};

export const Tabs = ({
  tabs,
  selectedTab,
  disabled,
  disableActiveTabHighlight,
  onChange,
  tabSx,
  tabIndicatorStyles,
}: Props) => (
  <MuiTabs
    value={selectedTab}
    onChange={onChange}
    sx={{
      bgcolor: (theme) => theme.palette.nym.nymWallet.background.grey,
      borderTop: '1px solid',
      borderBottom: '1px solid',
      borderColor: (theme) => theme.palette.nym.nymWallet.background.greyStroke,
      ...tabSx,
    }}
    textColor="inherit"
    TabIndicatorProps={{
      style: {
        opacity: disableActiveTabHighlight ? 0 : 1,
        ...tabIndicatorStyles,
      },
    }}
  >
    {tabs.map((tabName) => (
      <Tab key={tabName} label={tabName} sx={{ textTransform: 'capitalize' }} value={tabName} disabled={disabled} />
    ))}
  </MuiTabs>
);
