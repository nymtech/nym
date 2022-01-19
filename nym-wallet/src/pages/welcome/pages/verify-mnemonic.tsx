import React, { useEffect, useState } from 'react'
import { Button, Typography } from '@mui/material'
import { WordTiles, WordGuesses } from '../components/word-tiles'
import { TMnemonicObject } from '../types'

export const VerifyMnemonic = ({ words }: { words?: TMnemonicObject }) => {
  const [randomWords, setRandomWords] = useState<TMnemonicObject>()

  useEffect(() => {
    if (words) {
      const generatedRandomWords = getRandomEntriesFromObject(words, 4)
      setRandomWords(generatedRandomWords)
    }
  }, [words])

  return (
    <>
      <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Verify your mnemonic</Typography>
      <Typography sx={{ color: 'common.white' }}>Select the words from your mnmonic based on their order</Typography>
      <WordGuesses words={randomWords} />
      {randomWords && (
        <WordTiles words={randomWords} onClick={(arg) => alert(`word is: ${arg.word}. index is: ${arg.index}`)} />
      )}
      <Button variant="contained" sx={{ width: 300 }} size="large" disabled>
        Next
      </Button>
    </>
  )
}

const getRandomEntriesFromObject = (obj: { [key: string | number]: string }, numberOfEntries: number) => {
  let randomEntries: { [key: number]: string } = {}
  const keys = Object.keys(obj)

  while (Object.keys(randomEntries).length !== numberOfEntries) {
    const randomNumber = Math.floor(Math.random() * keys.length + 1)
    if (obj[randomNumber] !== undefined) {
      randomEntries[randomNumber] = obj[randomNumber]
    }
    delete obj[randomNumber]
  }
  return randomEntries
}
