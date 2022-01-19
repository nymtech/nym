import React, { useEffect, useState } from 'react'
import { Button, Typography } from '@mui/material'
import { WordTiles, HiddenWords } from '../components/word-tiles'
import { THiddenMnemonicWords, THiddenMnemonicWord, TMnemonicWord, TMnemonicWords } from '../types'
import { randomNumberBetween } from '../../../utils'

export const VerifyMnemonic = ({ words }: { words?: TMnemonicWords }) => {
  const [randomWords, setRandomWords] = useState<TMnemonicWords>()
  const [hiddenRandomWords, setHiddenRandomWords] = useState<THiddenMnemonicWords>()
  const [currentSelection, setCurrentSelection] = useState(0)

  useEffect(() => {
    if (words) {
      const randomWords = getRandomEntriesFromArray<TMnemonicWord>(words, 4)
      const withHiddenProperty = randomWords.map((word) => ({ ...word, hidden: true }))
      const shuffled = getRandomEntriesFromArray<THiddenMnemonicWord>(withHiddenProperty, 4)
      setRandomWords(randomWords)
      setHiddenRandomWords(shuffled)
    }
  }, [words])

  const revealWord = ({ name }: { name: string }) => {
    if (name === hiddenRandomWords![currentSelection].name) {
      setHiddenRandomWords((hiddenWords) =>
        hiddenWords?.map((word) => (word.name === name ? { ...word, hidden: false } : word)),
      )
      setCurrentSelection((current) => current + 1)
    }
  }

  if (randomWords && hiddenRandomWords) {
    return (
      <>
        <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Verify your mnemonic</Typography>
        <Typography sx={{ color: 'common.white' }}>Select the words from your mnmonic based on their order</Typography>
        <HiddenWords words={hiddenRandomWords} />
        <WordTiles words={randomWords} onClick={revealWord} />
        <Button variant="contained" sx={{ width: 300 }} size="large" disabled>
          Next
        </Button>
      </>
    )
  }
  return null
}

function getRandomEntriesFromArray<T>(arr: T[], numberOfEntries: number) {
  const init = [...arr]
  let randomEntries: T[] = []

  while (randomEntries.length !== numberOfEntries) {
    const rand = randomNumberBetween(0, init.length - 1)
    randomEntries.push(init[rand])
    init.splice(rand, 1)
  }

  return randomEntries
}
