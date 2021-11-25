import React, { useState } from 'react'
import { Button } from '@mui/material'
import { Check } from '@mui/icons-material'
import { green } from '@mui/material/colors'
import { clipboard } from '@tauri-apps/api'

const copy = (text: string): Promise<{ success: boolean; value: string }> => {
  return new Promise((resolve, reject) => {
    clipboard
      .writeText(text)
      .then(() => resolve({ success: true, value: text }))
      .catch((e) => reject({ success: false, value: 'Failed to copy: ' + e }))
  })
}

export const handleCopy = async ({
  text,
  cb,
}: {
  text: string
  cb: (success: boolean) => void
}) => {
  const res = await copy(text)
  if (res.success) {
    setTimeout(() => {
      cb(true)
    }, 750)
  } else {
    console.log(res.value)
  }
}

export const CopyToClipboard = ({ text }: { text: string }) => {
  const [copied, setCopied] = useState(false)

  const updateCopyStatus = (isCopied: boolean) => setCopied(isCopied)

  return (
    <Button
      size="small"
      variant={copied ? 'text' : 'outlined'}
      aria-label="save"
      data-testid="copy-button"
      onClick={() => handleCopy({ text, cb: updateCopyStatus })}
      endIcon={copied && <Check />}
      sx={copied ? { bgcolor: green[500], color: 'white' } : {}}
    >
      {copied ? 'Copied' : 'Copy'}
    </Button>
  )
}
