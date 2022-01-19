import React, { useEffect, useState } from 'react'
import { Button, Typography } from '@mui/material'
import { WordTiles, WordsSelection } from '../components/word-tiles'
import { TMnemonicArray } from '../types'
import { randomNumberBetween } from '../../../utils'

export const VerifyMnemonic = ({ words }: { words?: TMnemonicArray }) => {
  const [randomWords, setRandomWords] = useState<TMnemonicArray>()
  const [shuffledRandomWords, setShuffledRandomWords] = useState<TMnemonicArray>()

  useEffect(() => {
    if (words) {
      const randomWords = getRandomEntriesFromArray(words, 4)
      const shuffled = getRandomEntriesFromArray(randomWords, 4)
      setRandomWords(randomWords)
      setShuffledRandomWords(shuffled)
    }
  }, [words])
  return (
    <>
      <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Verify your mnemonic</Typography>
      <Typography sx={{ color: 'common.white' }}>Select the words from your mnmonic based on their order</Typography>
      <WordsSelection words={shuffledRandomWords} />
      <WordTiles words={randomWords} onClick={({ name, index }) => alert(`word is: ${name}. index is: ${index}`)} />
      <Button variant="contained" sx={{ width: 300 }} size="large" disabled>
        Next
      </Button>
    </>
  )
}

const getRandomEntriesFromArray = (arr: TMnemonicArray, numberOfEntries: number) => {
  const init = [...arr]
  let randomEntries: TMnemonicArray = []

  while (randomEntries.length !== numberOfEntries) {
    const rand = randomNumberBetween(0, init.length - 1)
    randomEntries.push(init[rand])
    init.splice(rand, 1)
  }

  return randomEntries
}
