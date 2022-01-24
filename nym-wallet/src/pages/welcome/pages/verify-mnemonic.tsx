import React, { useEffect, useState } from 'react'
import { Button } from '@mui/material'
import { WordTiles, HiddenWords } from '../components/word-tiles'
import { THiddenMnemonicWords, THiddenMnemonicWord, TMnemonicWord, TMnemonicWords } from '../types'
import { randomNumberBetween } from '../../../utils'
import { Title, Subtitle } from '../components'

const numberOfRandomWords = 4

export const VerifyMnemonic = ({
  mnemonicWords,
  onComplete,
}: {
  page: 'verify mnemonic'
  mnemonicWords?: TMnemonicWords
  onComplete: () => void
}) => {
  const [randomWords, setRandomWords] = useState<TMnemonicWords>()
  const [hiddenRandomWords, setHiddenRandomWords] = useState<THiddenMnemonicWords>()
  const [currentSelection, setCurrentSelection] = useState(0)

  useEffect(() => {
    if (mnemonicWords) {
      const randomWords = getRandomEntriesFromArray<TMnemonicWord>(mnemonicWords, numberOfRandomWords)
      const withHiddenProperty = randomWords.map((word) => ({ ...word, hidden: true }))
      const shuffled = getRandomEntriesFromArray<THiddenMnemonicWord>(withHiddenProperty, numberOfRandomWords)
      setRandomWords(randomWords)
      setHiddenRandomWords(shuffled)
    }
  }, [mnemonicWords])

  const revealWord = ({ name }: { name: string }) => {
    if (name === hiddenRandomWords![currentSelection].name) {
      setHiddenRandomWords((hiddenWords) =>
        hiddenWords?.map((word) => (word.name === name ? { ...word, hidden: false } : word)),
      )
      setRandomWords((randomWords) =>
        randomWords?.map((word) => (word.name === name ? { ...word, disabled: true } : word)),
      )
      setCurrentSelection((current) => current + 1)
    }
  }

  if (randomWords && hiddenRandomWords) {
    return (
      <>
        <Title title="Verify your mnemonic" />
        <Subtitle subtitle="Select the words from your mnmonic based on their order" />
        <HiddenWords mnemonicWords={hiddenRandomWords} />
        <WordTiles
          mnemonicWords={randomWords}
          onClick={currentSelection !== numberOfRandomWords ? revealWord : undefined}
        />
        <Button
          variant="contained"
          sx={{ width: 300 }}
          size="large"
          disabled={currentSelection !== numberOfRandomWords}
          onClick={onComplete}
        >
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
