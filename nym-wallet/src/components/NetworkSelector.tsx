import React, { useState, useContext } from 'react'
import { Button, List, ListItem, ListItemIcon, ListItemText, ListSubheader, Popover } from '@mui/material'
import { CheckSharp, KeyboardArrowDown } from '@mui/icons-material'
import { ClientContext } from '../context/main'
import { Network } from 'src/types'

const networks: { networkName: Network; name: string }[] = [
  { networkName: 'SANDBOX', name: 'Testnet Sandbox' },
  { networkName: 'QA', name: 'Testnet QA' },
]

export const NetworkSelector = () => {
  const { network, switchNetwork } = useContext(ClientContext)

  const [anchorEl, setAnchorEl] = useState<HTMLButtonElement | null>(null)

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setAnchorEl(event.currentTarget)
  }

  const handleClose = () => {
    setAnchorEl(null)
  }

  return (
    <>
      <Button
        variant="outlined"
        sx={{
          color: (theme) => `${theme.palette.nym.background.dark}`,
          border: (theme) => `1px solid ${theme.palette.nym.background.dark}`,
          '&:hover': { border: (theme) => `1px solid ${theme.palette.nym.background.dark}` },
        }}
        onClick={handleClick}
        endIcon={<KeyboardArrowDown sx={{ color: (theme) => `1px solid ${theme.palette.nym.background.dark}` }} />}
      >
        {networks.find((n) => n.networkName === network)?.name}
      </Button>
      <Popover
        open={Boolean(anchorEl)}
        anchorEl={anchorEl}
        anchorOrigin={{
          vertical: 'bottom',
          horizontal: 'left',
        }}
        onClose={handleClose}
      >
        <List>
          <ListSubheader>Network selection</ListSubheader>
          {networks.map(({ name, networkName }) => (
            <NetworkItem
              key={networkName}
              title={name}
              isSelected={networkName === network}
              onSelect={() => {
                handleClose()
                switchNetwork(networkName)
              }}
            />
          ))}
        </List>
      </Popover>
    </>
  )
}

const NetworkItem: React.FC<{ title: string; isSelected: boolean; onSelect: () => void }> = ({
  title,
  isSelected,
  onSelect,
}) => (
  <ListItem button onClick={onSelect}>
    <ListItemIcon>{isSelected && <CheckSharp color="success" />}</ListItemIcon>
    <ListItemText>{title}</ListItemText>
  </ListItem>
)
