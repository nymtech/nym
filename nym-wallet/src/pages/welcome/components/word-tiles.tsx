import React from 'react'
import { Card, CardHeader, Grid, Typography } from '@mui/material'
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
