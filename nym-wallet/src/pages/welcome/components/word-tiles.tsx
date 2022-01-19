import React from 'react'
import { Box, Card, CardHeader, Grid, Typography, Stack } from '@mui/material'
import { TMnemonicArray } from '../types'

export const WordTiles = ({
  words,
  showIndex,
  onClick,
}: {
  words?: TMnemonicArray
  showIndex?: boolean
  onClick?: ({ name, index }: { name: string; index: number }) => void
}) => {
  if (words) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {words.map(({ name, index }) => (
          <Grid item xs={2} key={index} onClick={() => onClick?.({ name, index })}>
            <WordTile word={name} index={showIndex ? index : undefined} onClick={!!onClick} />
          </Grid>
        ))}
      </Grid>
    )
  }

  return null
}

export const WordTile = ({ word, index, onClick }: { word: string; index?: number; onClick?: boolean }) => (
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

export const WordsSelection = ({ words }: { words?: TMnemonicArray }) => {
  if (words) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {words.map(({ name, index }) => (
          <Grid item xs={2} key={index}>
            <WordSelection index={index} />
          </Grid>
        ))}
      </Grid>
    )
  }
  return null
}

const WordSelection = ({ index }: { index: number }) => {
  return (
    <Stack spacing={2} alignItems="center">
      <Box borderBottom="1px solid #3A4053" sx={{ p: 2, width: '100%' }}></Box>
      <Typography>{index}.</Typography>
    </Stack>
  )
}
