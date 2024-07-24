import { Tab, Tabs as MuiTabs, Box } from '@mui/material';

export const Tabs: FCWithChildren<{
  tabs: string[];
  selectedTab: number;
  disabled: boolean;
  onChange: (event: React.SyntheticEvent, tab: number) => void;
}> = ({ tabs, selectedTab, disabled, onChange }) => (
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
  >
    {tabs.map((tabName) => (
      <Tab key={tabName} label={tabName} sx={{ textTransform: 'capitalize' }} disabled={disabled} />
    ))}
  </MuiTabs>
);

export const TabPanel: FCWithChildren = ({ children }) => <Box sx={{ p: 4 }}>{children}</Box>;
