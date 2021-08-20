import React, { useState } from 'react'
import { Button } from '@material-ui/core'
import { Check } from '@material-ui/icons'
import { green } from '@material-ui/core/colors'
import { clipboard } from '@tauri-apps/api'

const copy = (text: string): Promise<{ success: boolean; value: string }> => {
  return new Promise((resolve, reject) => {
    clipboard
      .writeText(text)
      .then(() => resolve({ success: true, value: text }))
      .catch((e) => reject({ success: false, value: 'Failed to copy: ' + e }))
  })
}

export const CopyToClipboard = ({ text }: { text: string }) => {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    setCopied(false)
    const res = await copy(text)
    console.log(res)
    if (res.success) {
      setCopied(true)
    } else {
      console.log(res.value)
    }
  }

  return (
    <Button
      size="small"
      variant="outlined"
      aria-label="save"
      onClick={handleCopy}
      endIcon={copied && <Check style={{ color: green[500] }} />}
    >
      {copied ? 'Copied' : 'Copy'}
    </Button>
  )
}
