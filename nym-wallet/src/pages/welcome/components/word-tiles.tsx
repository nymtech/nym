import React from 'react'
import { Box, Card, CardHeader, Grid, Typography, Stack, Fade } from '@mui/material'
import { THiddenMnemonicWord, THiddenMnemonicWords, TMnemonicWords } from '../types'

export const WordTiles = ({
  mnemonicWords,
  showIndex,
  onClick,
}: {
  mnemonicWords?: TMnemonicWords
  showIndex?: boolean
  onClick?: ({ name, index }: { name: string; index: number }) => void
}) => {
  if (mnemonicWords) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {mnemonicWords.map(({ name, index, disabled }) => (
          <Grid item xs={2} key={index} onClick={() => onClick?.({ name, index })}>
            <WordTile mnemonicWord={name} index={showIndex ? index : undefined} onClick={!!onClick} disabled={disabled}/>
          </Grid>
        ))}
      </Grid>
    )
  }

  return null
}

export const WordTile = ({
  mnemonicWord,
  index,
  disabled,
  onClick,
}: {
  mnemonicWord: string
  index?: number
  disabled?: boolean
  onClick?: boolean
}) => (
  <Card
    variant="outlined"
    sx={{
      background: '#151A2C',
      border: '1px solid #3A4053',
      cursor: onClick ? 'pointer' : 'default',
      opacity: disabled ? 0.2 : 1,
    }}
  >
    <CardHeader
      title={mnemonicWord}
      titleTypographyProps={{ sx: { fontWeight: 700 }, variant: 'body1', textAlign: index ? 'left' : 'center' }}
      avatar={
        index && (
          <Typography variant="caption" color={'#3A4053'}>
            {index}
          </Typography>
        )
      }
    />
  </Card>
)

export const HiddenWords = ({ mnemonicWords }: { mnemonicWords?: THiddenMnemonicWords }) => {
  if (mnemonicWords) {
    return (
      <Grid container spacing={3} justifyContent="center">
        {mnemonicWords.map((mnemonicWord) => (
          <Grid item xs={2} key={mnemonicWord.index}>
            <HiddenWord mnemonicWord={mnemonicWord} />
          </Grid>
        ))}
      </Grid>
    )
  }
  return null
}

const HiddenWord = ({ mnemonicWord }: { mnemonicWord: THiddenMnemonicWord }) => {
  return (
    <Stack spacing={2} alignItems="center">
      <Box borderBottom="1px solid #3A4053" sx={{ p: 2, width: '100%' }}>
        <Fade in={!mnemonicWord.hidden}>
          <Box>
            <WordTile mnemonicWord={mnemonicWord.name} />
          </Box>
        </Fade>
      </Box>
      <Typography>{mnemonicWord.index}.</Typography>
    </Stack>
  )
}
