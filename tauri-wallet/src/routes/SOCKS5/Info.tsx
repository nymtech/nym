import React, { useState } from 'react'
import { Box, IconButton, Popover, Theme, Typography } from '@material-ui/core'
import { HelpOutlineSharp } from '@material-ui/icons'
import { useTheme } from '@material-ui/styles'

export const Info = () => {
  const [anchorEl, setAnchorEl] = useState<HTMLButtonElement | null>()
  const open = Boolean(anchorEl)

  const theme: Theme = useTheme()
  return (
    <>
      <IconButton
        size="small"
        onClick={(event: React.MouseEvent<HTMLButtonElement>) => {
          setAnchorEl(() => event.currentTarget)
        }}
      >
        <HelpOutlineSharp />
      </IconButton>
      <Popover
        open={open}
        anchorEl={anchorEl}
        onClose={() => setAnchorEl(null)}
        transformOrigin={{
          vertical: 'top',
          horizontal: 'right',
        }}
      >
        <Box padding={theme.spacing(0.5)} maxWidth="400px">
          <Typography variant="h6" style={{ marginBottom: theme.spacing(1) }}>
            What is SOCKS5?
          </Typography>
          <Typography variant="body2">
            A SOCKS5 proxy is a private alternative to a VPN that protects the
            traffic within a specific source, such as an application. When you
            use a SOCKS5 proxy, data packets from the configured source are
            routed through a remote server. This server changes the IP address
            associated with these data packets before they reach their final
            destination
          </Typography>
        </Box>
      </Popover>
    </>
  )
}
