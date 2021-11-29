import React, { useEffect, useState } from 'react'
import { Button, IconButton, Tooltip } from '@mui/material'
import { Check, ContentCopy } from '@mui/icons-material'
import { clipboard } from '@tauri-apps/api'

export const CopyToClipboard = ({
  text = '',
  light,
  iconButton,
}: {
  text?: string
  light?: boolean
  iconButton?: boolean
}) => {
  const [copied, setCopied] = useState(false)

  const handleCopy = async (text: string) => {
    try {
      await clipboard.writeText(text)
      setCopied(true)
    } catch (e) {
      console.log('failed to copy: ' + e)
    }
  }

  useEffect(() => {
    let timer: NodeJS.Timeout
    if (copied) {
      timer = setTimeout(() => {
        setCopied(false)
      }, 2000)
    }
    return () => clearTimeout(timer)
  }, [copied])

  if (iconButton)
    return (
      <Tooltip title={!copied ? 'Copy' : 'Copied!'} leaveDelay={500}>
        <IconButton
          onClick={() => handleCopy(text)}
          size="small"
          sx={{
            color: (theme) =>
              light
                ? theme.palette.common.white
                : theme.palette.nym.background.dark,
          }}
        >
          {!copied ? (
            <ContentCopy fontSize="small" />
          ) : (
            <Check color="success" />
          )}
        </IconButton>
      </Tooltip>
    )

  return (
    <Button
      variant="outlined"
      color="inherit"
      sx={{
        color: (theme) =>
          light
            ? theme.palette.common.white
            : theme.palette.nym.background.dark,
        borderColor: (theme) =>
          light
            ? theme.palette.common.white
            : theme.palette.nym.background.dark,
      }}
      onClick={() => handleCopy(text)}
      endIcon={
        copied && (
          <Check sx={{ color: (theme) => theme.palette.success.light }} />
        )
      }
    >
      {!copied ? 'Copy' : 'Copied'}
    </Button>
  )
}
