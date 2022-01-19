import React from 'react'
import { Box, Card, CardHeader, Grid, Typography, Stack } from '@mui/material'
import { TMnemonicObject } from '../types'

export const WordTiles = ({
  words,
  showIndex,
  onClick,
}: {
  words?: TMnemonicObject
  showIndex?: boolean
  onClick?: ({ word, index }: { word: string; index: string }) => void
}) => {
  if (words) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {Object.entries(words).map(([index, word]) => (
          <Grid item xs={2} key={index} onClick={() => onClick?.({ word, index })}>
            <WordTile word={word} index={showIndex ? index : undefined} onClick={!!onClick} />
          </Grid>
        ))}
      </Grid>
    )
  }

  return null
}

export const WordTile = ({ word, index, onClick }: { word: string; index?: string; onClick?: boolean }) => (
  <Card
    variant="outlined"
    sx={{ background: '#151A2C', border: '1px solid #3A4053', cursor: onClick ? 'pointer' : 'default' }}
  >
    <CardHeader
      title={word}
      titleTypographyProps={{ sx: { fontWeight: 700 }, variant: 'body1', textAlign: index ? 'left' : 'center' }}
      avatar={
        index && (
          <Typography variant="caption" color="#3A4053">
            {index}
          </Typography>
        )
      }
    />
  </Card>
)

export const WordGuesses = ({ words }: { words?: TMnemonicObject }) => {
  if (words) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {Object.entries(words).map(([index, word]) => (
          <Grid item xs={2} key={index}>
            <WordGuess index={index} />
          </Grid>
        ))}
      </Grid>
    )
  }
  return null
}

const WordGuess = ({ index }: { index: string }) => {
  return (
    <Stack spacing={2} alignItems="center">
      <Box borderBottom="1px solid #3A4053" sx={{ p: 2, width: '100%' }}></Box>
      <Typography>{index}.</Typography>
    </Stack>
  )
}
