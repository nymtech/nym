import React, { useState } from 'react'
import { Card, CardHeader, Chip } from '@mui/material'
import { Box } from '@mui/system'

export const WordTile = () => {
  const [words, setWords] = useState(['hello', 'jellow', 'mellow', 'fellow', 'hello', 'jellow', 'mellow', 'fellow'])

  return (
    <Box display="flex" gap={1}>
      {words.map((w, i) => (
        <Chip label={`${i}. ${w}`} onClick={() => alert(w)} />
      ))}
    </Box>
  )
}
