import React, { createRef, forwardRef, useRef } from 'react'
import { Box, Card, CardHeader, Grid, Typography, Stack, Fade } from '@mui/material'
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWords } from '../types'

export const WordTiles = ({
  words,
  showIndex,
  onClick,
}: {
  words?: TMnemonicWords
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

export const WordTile = forwardRef(
  ({ word, index, onClick }: { word: string; index?: number; onClick?: boolean }, ref) => (
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
  ),
)

export const HiddenWords = ({ words }: { words?: THiddenMnemonicWords }) => {
  if (words) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {words.map((word) => (
          <Grid item xs={2} key={word.index}>
            <HiddenWord word={word} />
          </Grid>
        ))}
      </Grid>
    )
  }
  return null
}

const HiddenWord = ({ word }: { word: THiddenMnemonicWord }) => {
  return (
    <Stack spacing={2} alignItems="center">
      <Box borderBottom="1px solid #3A4053" sx={{ p: 2, width: '100%' }}>
        <Fade in={!word.hidden}>
          <Box>
            <WordTile word={word.name} />
          </Box>
        </Fade>
      </Box>
      <Typography>{word.index}.</Typography>
    </Stack>
  )
}
