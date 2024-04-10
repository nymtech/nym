import React from 'react'
import Box from '@mui/material/Box'
import MuiLink from '@mui/material/Link'
import Typography from '@mui/material/Typography'
import { useIsMobile } from '../hooks/useIsMobile'
import { NymVpnIcon } from '../icons/NymVpn'
import { Socials } from './Socials'
import Link from 'next/link'

export const Footer: FCWithChildren = () => {
  const isMobile = useIsMobile()

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
        width: '100%',
        height: 'auto',
        mt: 3,
        pt: 3,
        pb: 3,
      }}
    >
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'row',
          width: 'auto',
          justifyContent: 'center',
          alignItems: 'center',
          mb: 2,
        }}
      >
        <Box marginRight={1}>
          <Link href="http://nymvpn.com" target="_blank">
            <NymVpnIcon />
          </Link>
        </Box>

        <Socials isFooter />
      </Box>

      <Typography
        sx={{
          fontSize: 12,
          textAlign: isMobile ? 'center' : 'end',
          color: 'nym.muted.onDarkBg',
        }}
      >
        © {new Date().getFullYear()} Nym Technologies SA, all rights reserved
      </Typography>
    </Box>
  )
}
