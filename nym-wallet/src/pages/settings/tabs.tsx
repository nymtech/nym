import React from 'react'
import { Box } from '@mui/system'
import { Tab, Tabs as MuiTabs } from '@mui/material'
import { Overview } from './overview'

export const Tabs: React.FC<{
  tabs: string[]
  selectedTab: number
  onChange: (event: React.SyntheticEvent, tab: number) => void
}> = ({ tabs, selectedTab, onChange }) => (
  <MuiTabs
    value={selectedTab}
    onChange={onChange}
    sx={{ bgcolor: 'grey.200', borderTop: '1px solid', borderBottom: '1px solid', borderColor: 'grey.300' }}
    textColor="inherit"
  >
    {tabs.map((tabName, index) => (
      <Tab key={index} label={tabName} sx={{ textTransform: 'capitalize' }} />
    ))}
  </MuiTabs>
)

export const TabPanel: React.FC = ({ children }) => (
  <Box sx={{ p: 4 }}>
    <Overview />
    {children}
  </Box>
)
